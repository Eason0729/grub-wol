// TODO: fix mutability(RefCell maybe!)
use super::packet::{self, Packet, Packets};

use super::bootgraph::{self, *};

use indexmap::IndexMap;
use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use smol::net::{TcpListener, TcpStream};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
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

#[derive(Serialize, Deserialize)]
pub struct Server<'a> {
    machines: IndexMap<MacAddress, Machine<'a>>,
    #[serde(skip)]
    packets: Packets,
    // #[serde(skip)]
    // close: Cell<bool>,
    #[serde(skip)]
    unknown_packet: RefCell<RingBuffer<Packet<'a>, 4>>,
}

impl<'a> Server<'a> {
    pub fn new() -> Self {
        Self {
            machines: Default::default(),
            packets: Default::default(),
            unknown_packet: Default::default(),
        }
    }
    pub fn try_from_file() -> Self {
        todo!()
    }
    pub async fn listen(&'a self, socket: SocketAddr) -> Result<(), Error> {
        // TODO: graceful shutdown
        let listener = TcpListener::bind(socket).await?;
        loop {
            let (stream, addr) = listener.accept().await?;
            // TODO: error handling
            match self.connect_tcp(stream).await {
                Ok(_) => {}
                Err(err) => {
                    println!("{:?}", err);
                }
            };
        }
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
        if let Some(machine) = self.machines.get(&mac_address) {
            machine.connect(packet).await;
        } else {
            let mut unknown_packet = self.unknown_packet.borrow_mut();
            unknown_packet.push(packet);
        }
        Ok(())
    }
    pub fn list_machine(&'a self) -> Vec<&'a Machine> {
        self.machines.iter().map(|(_, v)| v).collect()
    }
    pub fn list_unknown(&self) -> Vec<MacAddress> {
        let mut unknown_packet = self.unknown_packet.borrow_mut();

        let mut result = vec![];
        for packet in unknown_packet.iter() {
            if let Ok(mac) = packet.get_mac() {
                result.push(mac);
            }
        }
        result
    }
    pub async fn new_machine(
        &'a mut self,
        mac: MacAddress,
        display_name: String,
    ) -> Result<bool, Error> {
        let mut unknown_packet = self.unknown_packet.borrow_mut();

        let packet = unknown_packet.pop(|item| match item.get_mac() {
            Ok(x) => x == mac,
            Err(_) => false,
        });

        if let Some(packet) = packet {
            let (machine, packet) = Machine::new(packet, display_name).await?;
            self.machines.insert(mac, machine);
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
    packet: RefCell<Option<Packet<'a>>>,
}

impl<'a> Machine<'a> {
    async fn connect(&self, packet: Packet<'a>) {
        let mut packet_wrapper = self.packet.borrow_mut();
        *packet_wrapper = Some(packet);
        todo!()
    }
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
            packet: RefCell::new(None),
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
