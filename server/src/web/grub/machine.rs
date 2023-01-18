use web;
// TODO: fix mutability(RefCell maybe!)
use super::packet::{self, Packet, Packets};

use super::bootgraph::{self, *};

use async_std::io::ReadExt;
use async_std::net;
use indexmap::IndexMap;
use log::{info, warn};
use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::{RwLock};
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
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.buffer.iter_mut()
    }
    fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct ServerSave<'a> {
    machines: IndexMap<MacAddress, Machine<'a>>,
}

impl<'a> ServerSave<'a> {
    fn new(path: &Path) -> Result<Self, Error> {
        Ok(if path.exists() && path.is_file() {
            let mut file = File::open(path)?;

            let buf = &mut Vec::new();
            file.read_to_end(buf)?;

            bincode::deserialize_from(file)?
        } else {
            Default::default()
        })
    }
    fn save(server: &Server, path: &Path) {
        let map = (&*server.machines.read().unwrap());
    }
}

pub struct Server<'a> {
    machines: RwLock<IndexMap<MacAddress, Machine<'a>>>,
    packets: Packets,
    unknown_packet: RwLock<RingBuffer<Packet<'a>, 4>>,
    listener: net::TcpListener,
}

impl<'a> Server<'a> {
    pub async fn new(socket: SocketAddr) -> Result<Server<'a>, Error> {
        let listener = net::TcpListener::bind(socket).await?;
        Ok(Self {
            machines: Default::default(),
            packets: Default::default(),
            unknown_packet: Default::default(),
            listener,
        })
    }
    pub async fn save(&self, path: &Path) -> Result<(), Error> {
        let save = (&*self.machines.read().unwrap()).clone();
        todo!()
    }
    pub async fn load(path: &Path) -> Result<Server<'a>, Error> {
        let bind_host = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let socket = net::SocketAddr::new(bind_host, protocal::SERVER_PORT);

        let save = if path.exists() && path.is_file() {
            info!("load server from {}", path.to_string_lossy());
            let mut file = File::open(path)?;

            let buf = &mut Vec::new();
            file.read_to_end(buf)?;

            bincode::deserialize_from::<_, ServerSave>(file)?
        } else {
            info!("initialize server with no machine connected");
            Default::default()
        };

        let mut server = Server::new(socket).await?;
        server.machines = RwLock::new(save.machines);

        Ok(server)
    }
    pub async fn tick(&'a self) -> Result<(), Error> {
        let (stream, _) = self.listener.accept().await?;
        match self.connect_tcp(stream).await {
            Ok(_) => {}
            Err(err) => {
                warn!("{:?}", err);
            }
        };
        Ok(())
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
    async fn connect_packet(&'a self, packet: Packet<'a>) -> Result<(), Error> {
        let mac_address = packet.get_mac()?;
        if let Some(machine) = self.machines.write().unwrap().get_mut(&mac_address) {
            machine.connect(packet);
        } else {
            let mut unknown_packet = self.unknown_packet.write().unwrap();
            unknown_packet.push(packet);
        }
        Ok(())
    }
    pub async fn new_machine(
        &'a self,
        mac: MacAddress,
        display_name: String,
    ) -> Result<bool, Error> {
        let mut unknown_packet = self.unknown_packet.write().unwrap();

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
    pub fn list_machine(&self) -> Result<web::MachineList, Error> {
        let mut machines = Vec::new();

        let machines_src = self.machines.read().unwrap();
        for (mac_address, machine) in machines_src.iter() {
            let current_os = machine.current_os()?.map(|os| os.id);
            machines.push(web::MachineInfo {
                state: match current_os {
                    Some(os) => web::MachineState::Up(os),
                    None => web::MachineState::Down,
                },
                mac_address:mac_address.clone(),
            });
        }
        let unknown_src = self.unknown_packet.write().unwrap();
        for packet in unknown_src.iter() {
            let mac_address=if let Ok(mac)=packet.get_mac(){
                mac
            }else{
                continue;
            };
            machines.push(web::MachineInfo {
                mac_address: mac_address.clone(),
                state:web::MachineState::Uninited
            });
        }

        Ok(web::MachineList { machines })
    }
    pub fn find_machine(&self, mac_address: &MacAddress) -> Result<Option<web::MachineInfo>, Error> {
        match self.machines.read().unwrap().get(mac_address) {
            Some(machine) => {
                let current_os = match machine.current_os()? {
                    Some(os) => Some(os.id),
                    None => None,
                };

                Ok(Some(web::MachineInfo {
                    mac_address: mac_address.clone(),
                    state: match current_os {
                        Some(os) => web::MachineState::Up(os),
                        None => web::MachineState::Down,
                    },
                }))
            }
            None => Ok(None),
        }
    }
    pub fn list_os(&self, mac_address: &MacAddress) -> Result<Option<web::OsList>, Error> {
        match self.machines.read().unwrap().get(mac_address) {
            Some(machine) => {
                let oss = machine
                    .list_os()
                    .map(|os| web::OsInfo {
                        display_name: os.display_name.clone(),
                        id: os.id,
                    })
                    .collect();
                Ok(Some(web::OsList { oss }))
            }
            None => Ok(None),
        }
    }
    pub async fn off(&self, mac_address: &MacAddress) -> Result<bool, Error> {
        match self.machines.write().unwrap().get_mut(mac_address) {
            Some(machine) => {
                machine.off().await?;
                info!("shutdown machine of 0x{:x?}", mac_address);
                Ok(true)
            }
            None => Ok(false),
        }
    }
    pub async fn boot(&self, mac_address: &MacAddress, os: bootgraph::OSId) -> Result<bool, Error> {
        match self.machines.write().unwrap().get_mut(mac_address) {
            Some(machine) => {
                info!("booting machine of 0x{:x?} to os(uid):{}", mac_address, os);
                machine.boot(os).await?;
                Ok(true)
            }
            None => Ok(false),
        }
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
    fn connect(&mut self, packet: Packet<'a>) -> Option<Packet<'a>> {
        match self.packet {
            Some(_) => Some(packet),
            None => {
                self.packet = Some(packet);
                None
            }
        }
    }
    async fn boot(&mut self, os: bootgraph::OSId) -> Result<(), Error> {
        let packet = self.packet.as_mut().ok_or(Error::ClientNotConnected)?;
        self.boot_graph
            .boot_into(os, packet, self.mac_address)
            .await?;
        Ok(())
    }
    async fn off(&mut self) -> Result<(), Error> {
        let packet = self.packet.as_mut().ok_or(Error::ClientNotConnected)?;
        self.boot_graph.off(packet, self.mac_address).await?;
        Ok(())
    }
    async fn new<'b>(
        packet: Packet<'b>,
        display_name: String,
    ) -> Result<(Machine, Packet<'_>), Error> {
        let mac_address = packet.get_mac()?;
        let id_counter = 1;
        let mut boot_graph = IntBootGraph::new(packet, id_counter).await?;

        boot_graph.try_yield().await?;

        let (boot_graph, packet, _) = boot_graph.disassemble();

        let machine = Machine {
            display_name,
            mac_address,
            boot_graph,
            packet: None,
        };

        Ok((machine, packet))
    }
    fn list_os(&self) -> impl Iterator<Item = &bootgraph::OS> {
        self.boot_graph.list_os()
    }
    fn current_os(&self) -> Result<Option<&OS>, Error> {
        Ok(match &self.packet {
            Some(packet) => match self.boot_graph.current_os(packet)? {
                OSState::Down => None,
                OSState::Up(os) => Some(os),
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
