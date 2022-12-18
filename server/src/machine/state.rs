use super::packet::{self, Packet, Packets};
use crate::machine::graph::Dijkstra;

use super::graph::{Graph, Node};

use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::collections::*;

pub type GrubSec = protocal::Integer;

#[derive(Clone, Ord, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum OS {
    Down,
    Up(OSInfo),
}

#[derive(Clone, Ord, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum BootMethod {
    WOL,
    Grub(protocal::Integer),
    Down,
}

impl BootMethod {
    pub async fn execute(&self, packet: &mut Packet<'_>) -> Result<(), packet::Error> {
        match self {
            BootMethod::WOL => todo!(),
            BootMethod::Grub(x) => {
                packet.boot_into(*x).await?;
                packet.wait_reconnect().await?;
            }
            BootMethod::Down => {
                packet.shutdown().await?;
            }
        }
        Ok(())
    }
    pub async fn follow(
        path: Vec<BootMethod>,
        packet: &mut Packet<'_>,
    ) -> Result<(), packet::Error> {
        for method in path {
            method.execute(packet).await?;
        }
        Ok(())
    }
    pub async fn follow_ref(
        path: Vec<&BootMethod>,
        packet: &mut Packet<'_>,
    ) -> Result<(), packet::Error> {
        for method in path {
            method.execute(packet).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Ord, Eq, Serialize, Deserialize)]
pub struct OSInfo {
    pub display_name: String,
    pub id: protocal::ID,
}

impl PartialOrd for OSInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl PartialEq for OSInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

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
    pub async fn add(&mut self, packet: &mut Packet<'_>) -> Result<(), packet::Error> {
        let id = packet.get_handshake_uid()?;
        let os_info = packet.get_os().await?;
        let grub_info = packet.grub_query().await?;

        if grub_info.is_empty() {
            return Ok(());
        }

        let ios = IntermediateOS {
            os: OS::Up(os_info),
            unknown_grub: grub_info,
            distance: 0,
        };

        self.ioss.push(ios);
        Ok(())
    }
    pub fn consume_closest<'a>(
        &mut self,
        graph: &'a mut Graph<OS, BootMethod>,
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
