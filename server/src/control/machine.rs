// TODO: fix mutability(RefCell maybe!)
use super::packet::{self, Packet, Packets};

use super::bootgraph::{self, *};

use indexmap::IndexMap;
use log::warn;
use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use smol::future::or;
use smol::lock::{Mutex, RwLock};
use smol::net::{TcpListener, TcpStream};
use std::fs::File;
use std::net::SocketAddr;
use std::{collections::*, io};

type MacAddress = [u8; 6];

struct RingBuffer<T, const SIZE: usize>
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
    fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(SIZE),
        }
    }
    fn push(&mut self, item: T) {
        if self.buffer.len() == SIZE {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }
    fn pop<F>(&mut self, f: F) -> Option<T>
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
    fn iter(&mut self) -> impl Iterator<Item = &mut T> {
        self.buffer.iter_mut()
    }
}

pub struct Server<'a> {
    machines: RwLock<IndexMap<MacAddress, Machine<'a>>>,
    packets: Packets,
    unknown_packet: Mutex<RingBuffer<Packet<'a>, 4>>,
    listener: TcpListener,
}

impl<'a> Server<'a> {
    pub async fn new(socket: SocketAddr) -> Result<Server<'a>, Error> {
        let listener = TcpListener::bind(socket).await?;
        Ok(Self {
            machines: Default::default(),
            packets: Default::default(),
            unknown_packet: Default::default(),
            listener,
        })
    }
    pub fn save(mut self, file: File) -> Result<(), Error> {
        // let data=bincode::serialize(self.machines.get_mut())?;

        // let mut org=vec![];
        // // file.read_to_end(&mut org);

        // file.
        // // if data!=org{
        // //     file.
        // // }
        todo!()
    }
    pub fn load() -> Self {
        todo!()
    }
    pub async fn listen(&'a self, handle: async_channel::Receiver<()>) -> Result<(), Error> {
        or(
            async {
                handle.recv().await.unwrap();
                Ok(())
            },
            async {
                loop {
                    if let Err(err) = self.tick().await {
                        return Err(err.into());
                    };
                }
            },
        )
        .await
    }
    pub async fn tick(&'a self) -> Result<(), Error> {
        let (stream, _) = self.listener.accept().await?;
        match self.connect_tcp(stream).await {
            Ok(_) => {}
            Err(err) => {
                warn!("grub-wol protocal server error at: {:?}", err);
            }
        };
        Ok(())
    }
    async fn connect_tcp(&'a self, stream: TcpStream) -> Result<(), Error> {
        let packet = match self.packets.connect(stream).await? {
            Some(x) => x,
            None => {
                return Ok(());
            }
        };
        self.connect_packet(packet).await
    }
    async fn connect_packet(&'a self, mut packet: Packet<'a>) -> Result<(), Error> {
        let mac_address = packet.get_mac()?;
        if let Some(machine) = self.machines.read().await.get(&mac_address) {
            machine.connect(packet).await;
        } else {
            let mut unknown_packet = self.unknown_packet.lock().await;
            unknown_packet.push(packet);
        }
        Ok(())
    }
    pub async fn list_machine(&'a self) -> Vec<&'a Machine> {
        // self.machines.read().await.iter().map(|(_, v)| v).collect()
        todo!()
    }
    pub async fn list_unknown(&self) -> Vec<MacAddress> {
        let mut unknown_packet = self.unknown_packet.lock().await;

        let mut result = vec![];
        for packet in unknown_packet.iter() {
            if let Ok(mac) = packet.get_mac() {
                result.push(mac);
            }
        }
        result
    }
    pub async fn new_machine(
        &'a self,
        mac: MacAddress,
        display_name: String,
    ) -> Result<bool, Error> {
        let mut unknown_packet = self.unknown_packet.lock().await;

        let packet = unknown_packet.pop(|item| match item.get_mac() {
            Ok(x) => x == mac,
            Err(_) => false,
        });

        if let Some(packet) = packet {
            let (machine, packet) = Machine::new(packet, display_name).await?;
            self.machines.write().await.insert(mac, machine);
            self.connect_packet(packet).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Machine<'a> {
    display_name: String,
    mac_address: MacAddress,
    boot_graph: BootGraph,
    #[serde(skip)]
    packet: Mutex<Option<Packet<'a>>>,
}

impl<'a> Machine<'a> {
    async fn connect(&self, packet: Packet<'a>) {
        let mut packet_wrapper = self.packet.lock().await;
        *packet_wrapper = Some(packet);
        todo!()
    }
    // async fn boot(&self){}
    async fn new<'b>(
        mut packet: Packet<'b>,
        display_name: String,
    ) -> Result<(Machine, Packet<'_>), Error> {
        let mac_address = packet.get_mac()?;
        let id_counter = 1;
        let mut boot_graph = IntBootGraph::new(packet, id_counter).await?;

        boot_graph.tick().await?;

        let (boot_graph, packet, _) = boot_graph.into_inner();

        let machine = Machine {
            display_name,
            mac_address,
            boot_graph,
            packet: Mutex::new(None),
        };

        Ok((machine, packet))
    }
    fn list_os(&self) -> Vec<&OS> {
        self.boot_graph.list_os().collect()
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
    #[error("Server Failure")]
    IoError(#[from] io::Error),
    #[error("Unable to save file")]
    BincodeError(#[from] bincode::Error),
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
