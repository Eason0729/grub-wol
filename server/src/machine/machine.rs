use super::packet::{self, Packet, Packets};

use super::bootgraph::{self, *};

use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use smol::net::TcpStream;
use std::collections::*;
use std::marker::PhantomData;

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
        F: Fn(&T) -> bool,
    {
        if let Some((i, _)) = self.buffer.iter().enumerate().find(|(i, item)| f(*item)) {
            self.buffer.remove(i)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Server<'a> {
    machines: BTreeMap<MacAddress, Machine<'a>>,
    #[serde(skip)] 
    packets: Packets,
    #[serde(skip)]
    close: bool,
    #[serde(skip)]
    unknown_packet: RingBuffer<Packet<'a>, 4>,
}

impl<'a> Server<'a> {
    pub async fn listen(){}
    async fn connect(&'a mut self,stream:TcpStream)->Result<(),Error>{
        let mut packet=match self.packets.connect(stream).await?{
            Some(x) => x,
            None => {return Ok(());},
        };

        let mac_address=packet.get_mac()?;
        if let Some(machine)=self.machines.get_mut(&mac_address){
            machine.connect(packet).await;
        }else{
            self.unknown_packet.push(packet);
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Machine<'a> {
    boot_graph: BootGraph,
    #[serde(skip)]
    packet: Option<Packet<'a>>,
}

impl<'a> Machine<'a> {
    async fn connect(&mut self,packet: Packet<'a>){
        todo!()
    }
    async fn new<'b>(packet: Packet<'b>) -> Result<(Machine, Packet<'_>), Error> {
        let id_counter = 1;
        let mut boot_graph = IntBootGraph::new(packet, id_counter).await?;

        boot_graph.tick().await?;

        let (boot_graph, packet, _) = boot_graph.into_inner();

        let machine = Machine {
            boot_graph,
            packet: None,
        };

        Ok((machine, packet))
    }
    fn list_os(&self) -> Vec<&OS> {
        self.boot_graph.list_os().collect()
    }
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Packet Error")]
    PacketError(#[from] packet::Error),
    #[error("BootGraph Error")]
    BootGraphError(bootgraph::Error),
    #[error("Undefined Client Behavior")]
    UndefinedClientBehavior,
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
