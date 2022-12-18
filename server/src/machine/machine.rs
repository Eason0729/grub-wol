use super::packet::{self, Packet, Packets};

use super::bootgraph::{*, self};

use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::collections::*;

type MacAddress = [u8; 6];

// #[derive(Serialize, Deserialize)]
// struct Server {
//     machines: Machines,
//     // #[serde(skip)]
//     // instances:Vec<MachineInstance>,
//     #[serde(skip)]
//     packets: Packets,
//     #[serde(skip)]
//     close: bool,
// }

// impl Server {
//     fn new() {}
//     async fn serve(&mut self) {}
//     async fn shutdown(&mut self) {}
// }

#[derive(Serialize, Deserialize)]
struct Machines {
    id_counter: protocal::ID, // note that id should start with 1
    machines: BTreeMap<MacAddress, Machine>,
}

impl Machines {
    async fn add_machine<'a>(&mut self, packet: &mut Packet<'a>) -> Result<(), Error> {
        // let boot_graph:BootGraph::
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
struct Machine {
    boot_graph: BootGraph,
}

impl Machine {
    async fn new<'a>(packet: Packet<'a>) -> Result<(Machine, Packet<'_>), Error> {
        let id_counter = 1;
        let mut boot_graph = IntBootGraph::new(packet, id_counter).await?;

        boot_graph.tick().await?;

        let (boot_graph, packet, _) = boot_graph.into_inner();

        let machine = Machine { boot_graph };

        Ok((machine, packet))
    }
}

struct MachineInstance<'a> {
    packet: Packet<'a>,
    machine: &'a Machine,
}

impl<'a> MachineInstance<'a> {
    fn new(machine: &'a Machine, packet: Packet<'a>) -> Self {
        Self { packet, machine }
    }
    // fn list_os(&self) -> Vec<&'a OSInfo> {
    //     self.machine
    //         .boot_graph
    //         .list_node()
    //         .filter_map(|state| match state {
    //             OS::Down => None,
    //             OS::Up(x) => Some(x),
    //         })
    //         .collect()
    // }
    // async fn boot_into(&mut self,os:OSInfo)->Result<(),Error>{
    //     // let packet=&mut self.packet;

    //     // let node=self.machine.boot_graph.find_node(&OS::Up(packet.get_os())).unwrap();
    //     // let dist=self.machine.boot_graph.find_node(&OS::Up(os)).unwrap();

    //     // let trace=self.machine.boot_graph.dijkstra(&node).trace(&dist).unwrap();
    //     // BootMethod::follow_ref(trace, packet);
    //     // Ok(())
    // }
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Packet Error")]
    PacketError(#[from] packet::Error),
    #[error("BootGraph Error")]
    BootGraphError(bootgraph::Error),
    #[error("Undefined Client Behavior")]
    UndefinedClientBehavior
}

impl From<bootgraph::Error> for Error{
    fn from(e: bootgraph::Error) -> Self {
        match e{
            bootgraph::Error::UndefinedClientBehavior => Self::UndefinedClientBehavior,
            bootgraph::Error::BadGraph => Self::BootGraphError(e),
            bootgraph::Error::PacketError(e) => Self::PacketError(e),
        }
    }
}