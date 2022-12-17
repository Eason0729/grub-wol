use super::graph::Graph;
use super::{graph::Dijkstra, ios::IntermediateOSs};
use crate::packet::{self, Packet, Packets};

use super::{ios, state::*};

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
        // reboot to ensure correct first-boot os
        packet.shutdown().await?;
        packet.wait_reconnect().await?;

        // issue id
        if packet.get_handshake_uid()? == 0 {
            let id = self.id_counter;
            self.id_counter += 1;
            packet.issue_id(id).await?;
        }

        let mut boot_graph: Graph<OS, BootMethod> = Graph::new();

        // construct shutdown->first-boot-os on boot_graph
        let node1 = boot_graph.add_node(OS::Down);
        let display_name = packet.os_query().await?.display_name;
        let id = packet.get_handshake_uid()?;
        let first_os = OS::Up(OSInfo { display_name, id });
        let node2 = boot_graph.add_node(first_os.clone());
        boot_graph.connect(node1, node2, BootMethod::WOL);
        boot_graph.connect(node2, node1, BootMethod::Down);

        let mut ioss = IntermediateOSs::new();
        ioss.add(packet).await?;

        let mut current_os = first_os;
        let mut current_node = node2;

        while !ioss.is_finish() {
            let (trace, dist_os, grub_info) = ioss.consume_closest(&mut boot_graph, &current_node);

            for method in trace {
                match method {
                    BootMethod::WOL => todo!(),
                    BootMethod::Grub(x) => {
                        packet.boot_into(x).await?;
                        packet.wait_reconnect().await?;
                    }
                    BootMethod::Down => {
                        packet.shutdown().await?;
                    }
                }
            }

            packet.boot_into(grub_info.grub_sec).await?;
            packet.wait_reconnect().await?;

            let id = packet.get_handshake_uid()?;
            let os_info = packet.os_query().await?;
            current_os = OS::from_info(os_info, id);
            current_node = boot_graph.find_node(&current_os).unwrap();

            let dist_node = boot_graph.find_node(&dist_os).unwrap();
            boot_graph.connect(
                dist_node,
                current_node,
                BootMethod::Grub(grub_info.grub_sec),
            );
        }
        
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
struct Machine {
    boot_graph: Graph<OS, usize>,
}

struct MachineInstance<'a> {
    id: protocal::ID,
    packet: Packet<'a>,
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Packet Error")]
    PacketError(#[from] packet::Error),
    #[error("Packet Error")]
    IntermediateError(#[from] ios::Error),
}
