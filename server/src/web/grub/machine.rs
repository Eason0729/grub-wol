use super::adaptor;
use tokio::net;
use tokio::sync::Mutex;
use website;
// TODO: fix mutability(RefCell maybe!)
use super::packet::{self, Packet, Packets};

use super::bootgraph::{self, *};
use super::serde::{Serde, ServerSave};

use indexmap::IndexMap;
use log::{info, warn};
use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::{collections::*, io};

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

pub struct Server<'a> {
    pub(super) machines: Mutex<IndexMap<MacAddress, Arc<Machine<'a>>>>,
    pub(super) packets: Packets,
    pub(super) unknown_packet: Mutex<RingBuffer<Packet<'a>, 4>>,
    pub(super) socket: SocketAddr,
}

impl<'a> Server<'a> {
    pub fn new(socket: SocketAddr) -> Server<'a> {
        Self {
            machines: Default::default(),
            packets: Default::default(),
            unknown_packet: Default::default(),
            socket,
        }
    }
    pub async fn save(&self, path: &Path) -> Result<(), Error> {
        ServerSave::save(&self, path).await;
        Ok(())
    }
    pub async fn load(path: &Path) -> Result<Server<'a>, Error> {
        Ok(ServerSave::load(path).await)
    }
    pub async fn start(&'a self) {
        let listener = net::TcpListener::bind(self.socket).await.unwrap();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            match self.connect_tcp(stream).await {
                Ok(_) => {}
                Err(err) => {
                    warn!("{:?}", err);
                }
            };
        }
    }
    async fn connect_tcp(&'a self, stream: net::TcpStream) -> Result<(), Error> {
        let packet = match self.packets.connect(stream).await? {
            Some(x) => x,
            None => {
                return Ok(());
            }
        };
        self.connect_packet(packet).await
    }
    async fn connect_packet(&self, packet: Packet<'a>) -> Result<(), Error> {
        let mac_address = packet.get_mac();
        if let Some(machine) = self.machines.lock().await.get_mut(&mac_address) {
            machine.connect(packet).await;
        } else {
            let mut unknown_packet = self.unknown_packet.lock().await;
            unknown_packet.push(packet);
        }
        Ok(())
    }
    pub async fn new_machine(
        &'a self,
        mac: MacAddress,
        display_name: String,
    ) -> Result<bool, Error> {
        let mut unknown_packet = self.unknown_packet.lock().await;

        let packet = unknown_packet.pop(|item| item.get_mac() == mac);

        if let Some(packet) = packet {
            let (machine, packet) = Machine::new(packet, display_name).await?;
            self.machines.lock().await.insert(mac, Arc::new(machine));
            self.connect_packet(packet).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn get_machine(&'a self, mac_address: &'a [u8; 6]) -> Option<Arc<Machine<'a>>> {
        self.machines
            .lock()
            .await
            .get(mac_address)
            .map(|a| a.clone())
    }
    pub async fn list_os<'b>(&'b self, mac_address: &'b [u8; 6]) -> adaptor::OsListAdaptor<'b>where 'b :'a {
        adaptor::OsListAdaptor {
            machine: self.get_machine(mac_address).await,
        }
    }
    pub async fn info_machine(
        &'a self,
        mac_address: &'a [u8; 6],
    ) -> adaptor::MachineInfoAdaptor<'a> {
        adaptor::MachineInfoAdaptor {
            machine: self.get_machine(mac_address).await,
        }
    }
    pub fn list_machine(&'a self) -> adaptor::MachineListAdaptor<'a, 'a> {
        adaptor::MachineListAdaptor { server: self }
    }
    pub async fn boot(
        &'a self,
        os: website::OSState,
        mac_address: &'a [u8; 6],
    ) -> adaptor::BootAdaptor {
        adaptor::BootAdaptor {
            os,
            machine: self.get_machine(mac_address).await,
        }
    }
    pub async fn init_machine(
        &'a self,
        mac_address: [u8; 6],
        display_name: String,
    ) -> adaptor::NewMachineAdaptor<'a, 'a> {
        adaptor::NewMachineAdaptor {
            display_name,
            mac_address,
            server: self,
        }
    }
}

pub struct Machine<'a> {
    pub(super) display_name: Mutex<String>,
    pub(super) mac_address: MacAddress,
    pub(super) boot_graph: BootGraph,
    pub(super) packet: Mutex<Option<Packet<'a>>>,
}

impl<'a> Machine<'a> {
    pub(super) async fn connect(&self, packet: Packet<'a>) -> Option<Packet<'a>> {
        let mut current_packet = self.packet.lock().await;
        match &*current_packet {
            Some(_) => Some(packet),
            None => {
                *current_packet = Some(packet);
                None
            }
        }
    }
    pub(super) async fn new<'b>(
        packet: Packet<'b>,
        display_name: String,
    ) -> Result<(Machine, Packet<'_>), Error> {
        let mac_address = packet.get_mac();
        let id_counter = 1;
        let mut boot_graph = IntBootGraph::new(packet, id_counter).await?;

        boot_graph.try_yield().await?;

        let (boot_graph, packet, _) = boot_graph.disassemble();

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
                OSState::Down => None,
                OSState::Up(os) => Some(os.id),
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
    #[error("Client not connetced")]
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

struct Tester<G>
where
    G: Sync,
{
    a: PhantomData<G>,
}

type tester_result<'a> = Tester<Server<'a>>;
