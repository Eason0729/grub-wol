use super::packet::{self, TcpPacket, TcpPackets};
use super::{adaptor, api};
use async_std::future::timeout;
use async_std::{net, process};
use async_std::sync::Mutex;
use async_std::task::spawn;

use super::bootgraph::{self, *};
use super::serde::{Serde, ServerSave};

use indexmap::IndexMap;
use proto::prelude as protocal;
use core::time;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::{collections::*, io};

lazy_static! {
    static ref SAVE_PATH: &'static Path = Path::new("./");
}

type MacAddress = [u8; 6];

pub(super) struct RingBuffer<T, const SIZE: usize>
where
    T: Sized,
{
    buffer: VecDeque<T>,
}

impl<T, const SIZE: usize> Default for RingBuffer<T, SIZE> {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
        }
    }
}

impl<T, const SIZE: usize> RingBuffer<T, SIZE>
where
    T: Sized,
{
    pub(super) fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(SIZE),
        }
    }
    pub(super) fn push(&mut self, item: T) {
        if self.buffer.len() == SIZE {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }
    pub(super) fn pop<F>(&mut self, f: F) -> Option<T>
    where
        F: Fn(&mut T) -> bool,
    {
        let mut ans = None;
        for i in 0..self.buffer.len() {
            if f(&mut self.buffer[i]) {
                ans = Some(i);
                break;
            }
        }
        if let Some(i) = ans {
            self.buffer.remove(i)
        } else {
            None
        }
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }
}

pub struct Server {
    pub(super) machines: Mutex<IndexMap<MacAddress, Arc<Machine>>>,
    pub(super) packets: TcpPackets,
    pub(super) unknown_packet: Mutex<RingBuffer<TcpPacket, 4>>,
    pub(super) socket: SocketAddr,
}

impl Server {
    pub fn new(socket: SocketAddr) -> Server {
        Self {
            machines: Default::default(),
            packets: Default::default(),
            unknown_packet: Default::default(),
            socket,
        }
    }
    pub async fn save(&self) -> Result<(), Error> {
        log::info!("Backing up Grub server");
        ServerSave::save(&self, &SAVE_PATH).await;
        Ok(())
    }
    pub async fn load(path: &Path) -> Result<Server, Error> {
        Ok(ServerSave::load(path).await)
    }
    pub async fn start(self_: Arc<Self>) {
        log::info!("Creating autosave thread");
        let self_c=self_.clone();

        ctrlc::set_handler(move||{
            let self_=self_c.clone();
            spawn(async move{
                match timeout(time::Duration::from_secs(16), self_.save()).await{
                    Ok(x) => x.unwrap(),
                    Err(err) => log::error!("Timeout saving server file {:?}",err),
                };
                process::exit(1);
            });
        }).expect("cannot recieve sigterm");
        log::info!("Starting Grub server");
        let listener = net::TcpListener::bind(self_.socket).await.unwrap();
        loop {
            let (stream, socket) = listener.accept().await.unwrap();
            log::debug!(
                "Client from socket({}) is trying to connect to Grub server",
                socket
            );
            match self_.connect_tcp(stream).await {
                Ok(_) => {}
                Err(err) => {
                    log::warn!("{:?}", err);
                }
            };
        }
    }
    async fn connect_tcp(&self, stream: net::TcpStream) -> Result<(), Error> {
        match self.packets.connect(stream).await? {
            Some(packet) => {
                self.connect_packet(packet).await?;
            },
            None => {
            }
        };
        return Ok(());
    }
    async fn connect_packet(&self, packet: TcpPacket) -> Result<(), Error> {
        let mac_address = packet.get_mac_address().clone();
        if let Some(machine) = self.machines.lock().await.get_mut(&mac_address) {
            machine.connect(packet).await;
        } else {
            let mut unknown_packet = self.unknown_packet.lock().await;
            unknown_packet.push(packet);
        }
        Ok(())
    }
    pub(super) async fn new_machine(
        &self,
        mac: MacAddress,
        display_name: String,
    ) -> Result<bool, Error> {
        log::debug!("initializing new machine of mac address({:x?})", &mac);
        let mut unknown_packet = self.unknown_packet.lock().await;

        let packet = unknown_packet.pop(|item| *item.get_mac_address() == mac);

        if let Some(packet) = packet {
            let (machine, packet) = Machine::new(packet, display_name).await?;
            self.machines.lock().await.insert(mac, Arc::new(machine));
            self.connect_packet(packet).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn get_machine(&self, mac_address: &[u8; 6]) -> Option<Arc<Machine>> {
        self.machines
            .lock()
            .await
            .get(mac_address)
            .map(|a| a.clone())
    }
    pub async fn list_os(&self, mac_address: &[u8; 6]) -> adaptor::OsListAdaptor {
        adaptor::OsListAdaptor {
            machine: self.get_machine(mac_address).await,
        }
    }
    pub async fn info_machine(&self, mac_address: &[u8; 6]) -> adaptor::MachineInfoAdaptor {
        adaptor::MachineInfoAdaptor {
            machine: self.get_machine(mac_address).await,
        }
    }
    pub fn list_machine(&self) -> adaptor::MachineListAdaptor {
        adaptor::MachineListAdaptor { server: self }
    }
    pub async fn boot(&self, os: api::OsStatus, mac_address: &[u8; 6]) -> adaptor::BootAdaptor {
        adaptor::BootAdaptor {
            os,
            machine: self.get_machine(mac_address).await,
        }
    }
    pub async fn init_machine<'a>(
        &'a self,
        mac_address: [u8; 6],
        display_name: String,
    ) -> adaptor::NewMachineAdaptor<'a> {
        adaptor::NewMachineAdaptor {
            display_name,
            mac_address,
            server: self,
        }
    }
}

pub struct Machine {
    pub(super) display_name: Mutex<String>,
    pub(super) mac_address: MacAddress,
    pub(super) boot_graph: BootGraph,
    pub(super) packet: Mutex<Option<TcpPacket>>,
}

impl Machine {
    pub(super) async fn connect(&self, packet: TcpPacket) -> Option<TcpPacket> {
        let display_name=self.display_name.lock().await;
        log::trace!("machine {} connected",display_name);

        let mut current_packet = self.packet.lock().await;
        match &*current_packet {
            Some(_) => Some(packet),
            None => {
                *current_packet = Some(packet);
                None
            }
        }
    }
    pub(super) async fn new(
        packet: TcpPacket,
        display_name: String,
    ) -> Result<(Machine, TcpPacket), Error> {
        let mac_address = packet.get_mac_address().clone();
        let (boot_graph,packet) = BootGraph::new(packet).await?;

        log::info!("finish machine with name {}", display_name);
        let machine = Machine {
            display_name: Mutex::new(display_name),
            mac_address,
            boot_graph,
            packet: Mutex::new(None),
        };

        Ok((machine, packet))
    }
    pub(super) async fn current_os(&self) -> Result<Option<protocal::ID>, Error> {
        let mut packet1 = self.packet.lock().await;
        let packet = &mut *packet1;
        Ok(match packet {
            Some(packet) => match self.boot_graph.current_os(packet).await? {
                OsStatus::Down => None,
                OsStatus::Up(os) => Some(os),
            },
            None => None,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Io Error")]
    PacketError(#[from] packet::Error),
    #[error("Bugs occurs in either graph logic or Host")]
    BootGraphError(bootgraph::Error),
    #[error("Host didn't follow protocal")]
    UndefinedClientBehavior,
    #[error("client already connected")]
    ClientConnected,
    #[error("Server Failure")]
    IoError(#[from] io::Error),
    #[error("Unable to save file")]
    BincodeError(#[from] bincode::Error),
    #[error("Client not connected")]
    ClientNotConnected,
}

impl From<bootgraph::Error> for Error {
    fn from(e: bootgraph::Error) -> Self {
        match e {
            bootgraph::Error::UndefinedClientBehavior => Self::UndefinedClientBehavior,
            bootgraph::Error::BadGraph => Self::BootGraphError(e),
            bootgraph::Error::PacketError(e) => Self::PacketError(e),
        }
    }
}
