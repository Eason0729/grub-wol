// use crate::packet::{Packet, Packets, self};

// use super::graph::Graph;

// use proto::prelude as protocal;
// use serde::{Deserialize, Serialize};
// use std::collections::*;

// type NodeId = usize;
// type MacAddress = [u8; 6];

// #[derive(Serialize, Deserialize)]
// struct GrubAction {
//     grub_sec: usize,
// }

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

// #[derive(Serialize, Deserialize)]
// struct Machines {
//     id_counter:protocal::ID,// note that id should start with 1
//     machines: BTreeMap<MacAddress, Machine>,
// }

// impl Machines {
//     async fn add_machine<'a>(&mut self,packet:&mut Packet<'a>)->Result<(),Error>{

//         async fn issue_id<'b>(s:&mut Machines,packet:&mut Packet<'b>)->Result<(),Error>{
//             if packet.get_handshake_uid()?==0{
//                 let id=s.id_counter;
//                 s.id_counter+=1;
//                 packet.issue_id(id).await?;
//                 packet.fake_handshake_uid(id);
//             }
//             Ok(())
//         }

//         let mut queue=VecDeque::new();
//         let boot_graph: Graph<OS, usize>=Graph::new();

//         issue_id(self,packet).await?;

//         queue.push_back(packet.get_handshake_uid()?);

//         while !queue.is_empty(){

//             let current_os=queue.pop_front().unwrap();

//             issue_id(self,packet).await?;
//         }

//         todo!()
//     }
// }

// #[derive(Serialize, Deserialize)]
// struct Machine {
//     boot_graph: Graph<OS, usize>,
// }

// #[derive(Ord, PartialOrd, Eq, Serialize, Deserialize)]
// struct OS {
//     display_name: String,
//     id: protocal::ID,
// }

// impl PartialEq for OS{
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }

// struct MachineInstance<'a> {
//     id: protocal::ID,
//     packet: Packet<'a>,
// }

// #[derive(thiserror::Error, Debug)]
// enum Error {
//     #[error("Packet Error")]
//     PacketError(#[from]packet::Error)
// }
