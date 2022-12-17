use crate::packet::{self, Packet};

use super::graph::{Graph, Node};
use super::state::*;

use proto::prelude as protocal;

type BootGraph = Graph<OS, BootMethod>;

// struct BG<'a> {
//     graph: Graph<OS, BootMethod>,
//     packet: Packet<'a>,
// }

// impl<'a> BG<'a> {
//     async fn follow(&mut self, path: Vec<BootMethod>) -> Result<(), Error> {
//         for method in path {
//             match method {
//                 BootMethod::WOL => todo!(),
//                 BootMethod::Grub(x) => {
//                     self.packet.boot_into(x).await?;
//                     self.packet.wait_reconnect().await?;
//                 }
//                 BootMethod::Down => {
//                     self.packet.shutdown().await?;
//                 }
//             }
//         }
//         Ok(())
//     }
//     async fn new(mut packet: Packet<'a>) -> Result<BG<'a>, Error> {
//         let mut boot_graph: Graph<OS, BootMethod> = Graph::new();
//         // construct shutdown->first-boot-os on boot_graph
//         let node1 = boot_graph.add_node(OS::Down);
//         let display_name = packet.os_query().await?.display_name;
//         let id = packet.get_handshake_uid()?;
//         let first_os = OS::Up(OSInfo { display_name, id });
//         let node2 = boot_graph.add_node(first_os.clone());
//         boot_graph.connect(node1, node2, BootMethod::WOL);
//         boot_graph.connect(node2, node1, BootMethod::Down);

//         Ok(todo!())
//     }
// }

struct IntermediateOS {
    os: OS,
    unknown_grub: Vec<protocal::host::GrubInfo>,
    distance: usize,
}

pub struct IntermediateOSs {
    ioss: Vec<IntermediateOS>,
}

impl IntermediateOSs {
    pub fn new() -> Self {
        Self { ioss: vec![] }
    }
    pub fn is_finish(&self) -> bool {
        self.ioss.is_empty()
    }
    pub async fn add(&mut self, packet: &mut Packet<'_>) -> Result<(), Error> {
        let id = packet.get_handshake_uid()?;
        let os_info = packet.os_query().await?;
        let grub_info = packet.grub_query().await?;

        if grub_info.is_empty() {
            return Ok(());
        }

        let ios = IntermediateOS {
            os: OS::from_info(os_info, id),
            unknown_grub: grub_info,
            distance: 0,
        };

        self.ioss.push(ios);
        Ok(())
    }
    pub fn consume_closest<'a>(
        &mut self,
        graph: &'a mut BootGraph,
        root: &Node,
    ) -> (Vec<BootMethod>, OS, protocal::host::GrubInfo) {
        // compute distances
        let dijkstra = graph.dijkstra(root);
        for ios in &mut self.ioss {
            let dist = graph.find_node(&ios.os).unwrap();
            let distance = dijkstra.to(&dist).unwrap();
            ios.distance = distance;
        }
        // find closest
        let min_distance = self.ioss.iter().map(|ios| ios.distance).min().unwrap();
        let mut min_index = 0;
        loop {
            if self.ioss[min_index].distance == min_distance {
                break;
            }
            min_index += 1;
        }
        // consume closest
        let closest_ios = &mut self.ioss[min_index];
        let os = closest_ios.os.clone();
        let os_node = graph.find_node(&os).unwrap();
        // get grub_info
        let grub_info = closest_ios.unknown_grub.pop().unwrap();
        // remove IOS if empty
        if closest_ios.unknown_grub.is_empty() {
            self.ioss.swap_remove(min_index);
        }
        // get trace
        let trace = dijkstra
            .trace(&os_node)
            .unwrap()
            .into_iter()
            .map(|e| e.clone())
            .collect();

        (trace, os, grub_info)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Packet Error")]
    PacketError(#[from] packet::Error),
}
