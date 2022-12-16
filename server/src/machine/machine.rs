use crate::packet::{self, Packet, Packets};

use super::graph::Graph;

use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::collections::*;

type NodeId = usize;
type MacAddress = [u8; 6];

#[derive(Serialize, Deserialize)]
struct GrubAction {
    grub_sec: usize,
}

#[derive(Serialize, Deserialize)]
struct Server {
    machines: Machines,
    // #[serde(skip)]
    // instances:Vec<MachineInstance>,
    #[serde(skip)]
    packets: Packets,
    #[serde(skip)]
    close: bool,
}

impl Server {
    fn new() {}
    async fn serve(&mut self) {}
    async fn shutdown(&mut self) {}
}

#[derive(Serialize, Deserialize)]
struct Machines {
    id_counter: protocal::ID, // note that id should start with 1
    machines: BTreeMap<MacAddress, Machine>,
}

impl Machines {
    async fn add_machine<'a>(&mut self, packet: &mut Packet<'a>) -> Result<(), Error> {
        async fn issue_id<'b>(s: &mut Machines, packet: &mut Packet<'b>) -> Result<(), Error> {
            if packet.get_handshake_uid()? == 0 {
                let id = s.id_counter;
                s.id_counter += 1;
                packet.issue_id(id).await?;
            }
            Ok(())
        }

        // reboot to ensure correct first-boot os
        packet.shutdown().await?;
        packet.wait_reconnect().await?;

        // construct boot_graph
        let mut boot_graph: Graph<Option<OS>, protocal::host::GrubDescription> = Graph::new();

        issue_id(self, packet).await?;

        let root=boot_graph.add_node(None);
        boot_graph.bfs(&root);
        
        let grub_list = packet.grub_query().await?;

        grub_list.into_iter().for_each(|grub|{
            // however, you can't change node in place
            let dist=boot_graph.add_node(None);
            boot_graph.connect(root, dist, grub);
        });

        todo!()
    }
}

#[derive(Serialize, Deserialize)]
struct Machine {
    boot_graph: Graph<OS, usize>,
}

#[derive(Ord, PartialOrd, Eq, Serialize, Deserialize)]
struct OS {
    display_name: String,
    id: protocal::ID,
}

impl PartialEq for OS {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

struct MachineInstance<'a> {
    id: protocal::ID,
    packet: Packet<'a>,
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Packet Error")]
    PacketError(#[from] packet::Error),
}
