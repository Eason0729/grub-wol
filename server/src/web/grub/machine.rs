use super::api;
// TODO: fix mutability(RefCell maybe!)
use super::packet::{self, Packet, Packets};

use super::bootgraph::{self, *};

use indexmap::IndexMap;
use log::warn;
use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use smol::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{RwLock, Mutex};
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
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        // let mut file=File::open(path)?;
        // let data=&*self.machines.read().unwrap();
        // let machines = bincode::serialize(&data)?;

        // let mut data = vec![];
        // file.read_to_end(&mut data)?;

        // if machines!=data{
        //     File::create(path)?.write_all(&data)?;
        // }

        // Ok(())
        todo!()
    }
    pub async fn load(socket: SocketAddr,path: &Path) -> Result<Server<'a>, Error> {
        // let mut file=File::open(path)?;

        // let mut data = vec![];
        // file.read_to_end(&mut data)?;
        // let machines = bincode::deserialize(&data)?;

        // let listener = TcpListener::bind(socket).await?;

        // Ok(Self {
        //     machines:RwLock::new(machines),
        //     packets: Default::default(),
        //     unknown_packet: Default::default(),
        //     listener
        // })
        todo!()
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
        if let Some(machine) = self.machines.write().unwrap().get_mut(&mac_address) {
            machine.connect(packet);
        } else {
            let mut unknown_packet = self.unknown_packet.lock().unwrap();
            unknown_packet.push(packet);
        }
        Ok(())
    }
    // pub fn list_machine(&'a self) -> Vec<&'a Machine> {
    //     // self.machines.read().await.iter().map(|(_, v)| v).collect()
    //     todo!()
    // }
    // pub fn list_unknown(&self) -> Result<Vec<MacAddress>, Error> {
    //     let mut unknown_packet = self.unknown_packet.lock().unwrap();

    //     let mut result = vec![];
    //     for packet in unknown_packet.iter() {
    //         if let Ok(mac) = packet.get_mac() {
    //             result.push(mac);
    //         }
    //     }
    //     Ok(result)
    // }
    pub async fn new_machine(
        &'a self,
        mac: MacAddress,
        display_name: String,
    ) -> Result<bool, Error> {
        let mut unknown_packet = self.unknown_packet.lock().unwrap();

        let packet = unknown_packet.pop(|item| match item.get_mac() {
            Ok(x) => x == mac,
            Err(_) => false,
        });

        if let Some(packet) = packet {
            let (machine, packet) = Machine::new(packet, display_name).await?;
            self.machines.write().unwrap().insert(mac, machine);
            self.connect_packet(packet).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn list_machine(&self)->Vec<u8>{
        let mac_address=self.machines.read().unwrap().iter();

        todo!()
    }
    pub fn find_machine(){

    }
    pub fn list_os(){

    }
    pub fn find_os(){

    }
}

#[derive(Serialize, Deserialize)]
pub struct Machine<'a> {
    display_name: String,
    mac_address: MacAddress,
    boot_graph: BootGraph,
    #[serde(skip)]
    packet: Option<Packet<'a>>,
}

impl<'a> Machine<'a> {
    fn connect(&mut self, packet: Packet<'a>) ->Option<Packet<'a>>{
        match self.packet{
            Some(_) => Some(packet),
            None => {
                self.packet=Some(packet);
                None
            },
        }
    }
    async fn boot(&mut self,os:bootgraph::OSId)->Result<(),Error>{
        let packet=self.packet.as_mut().ok_or(Error::ClientNotConnected)?;
        self.boot_graph.boot_into(os, packet, self.mac_address).await?;
        Ok(())
    }
    async fn new<'b>(
        mut packet: Packet<'b>,
        display_name: String,
    ) -> Result<(Machine, Packet<'_>), Error> {
        let mac_address = packet.get_mac()?;
        let id_counter = 1;
        let mut boot_graph = IntBootGraph::new(packet, id_counter).await?;

        boot_graph.tick().await?;

        let (boot_graph, packet, _) = boot_graph.disassemble();

        let machine = Machine {
            display_name,
            mac_address,
            boot_graph,
            packet: None,
        };

        Ok((machine, packet))
    }
    fn list_os(&self) ->impl Iterator<Item = &bootgraph::OS> {
        self.boot_graph.list_os()
    }
    fn find_os(&self,os:bootgraph::OSId)->Option<&bootgraph::OS>{
        self.boot_graph.find_os(os)
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
