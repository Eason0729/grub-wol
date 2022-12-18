use super::packet::{self, Packet, Packets};
use crate::machine::graph::Dijkstra;

use super::graph::{Graph, Node};

use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::collections::*;

pub type GrubSec = protocal::Integer;

trait IntoLow {
    type Low;
    fn into_low(&self) -> Self::Low;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Deserialize, Serialize)]
pub struct OS {
    id: protocal::ID,
    display_name: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct LowOS {
    id: protocal::ID,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct HighOS {
    id: protocal::ID,
    display_name: String,
    unknown_edge: Vec<GrubSec>,
}

impl IntoLow for HighOS {
    type Low = LowOS;

    fn into_low(&self) -> Self::Low {
        LowOS { id: self.id }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
enum OSState<T> {
    Down,
    Up(T),
}

impl<T> IntoLow for OSState<T>
where
    T: IntoLow,
{
    type Low = OSState<T::Low>;

    fn into_low(&self) -> Self::Low {
        match self {
            OSState::Down => OSState::Down,
            OSState::Up(x) => OSState::Up(x.into_low()),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Deserialize, Serialize)]
pub enum BootMethod {
    WOL,
    Grub(GrubSec),
    Shutdown,
}

impl BootMethod {
    pub async fn execute(&self, packet: &mut Packet<'_>) -> Result<(), packet::Error> {
        match self {
            BootMethod::WOL => todo!(),
            BootMethod::Grub(x) => {
                packet.boot_into(*x).await?;
                packet.wait_reconnect().await?;
            }
            BootMethod::Shutdown => {
                packet.shutdown().await?;
            }
        }
        Ok(())
    }
}

pub struct IntBootGraph<'a> {
    // TODO: solve graph trouble about down status
    graph: Graph<OSState<LowOS>, BootMethod>,
    packet: Packet<'a>,
    unknown_os: Vec<HighOS>,
    ioss: Vec<HighOS>,
    id_counter: &'a mut protocal::ID,
}

impl<'a> IntBootGraph<'a> {
    pub async fn new(
        packet: Packet<'a>,
        id_counter: &'a mut protocal::ID,
    ) -> Result<IntBootGraph<'a>, packet::Error> {
        let mut self_ = IntBootGraph {
            graph: Graph::new(),
            packet,
            unknown_os: vec![],
            id_counter,
            ioss: vec![],
        };

        // reboot to ensure correct first-boot os
        self_.packet.shutdown().await?;
        self_.packet.wait_reconnect().await?;

        // construct shutdown->first-boot-os on boot_graph
        let shutdown_node = self_.graph.add_node(OSState::Down);
        let fboot_os = self_.issue_id().await?.unwrap();
        let fboot_node = self_
            .graph
            .add_node(OSState::Up(fboot_os.clone()).into_low());

        self_
            .graph
            .connect(shutdown_node, fboot_node, BootMethod::WOL);

        // add fboot os to unknown
        self_.unknown_os.push(fboot_os);

        Ok(self_)
    }
    /// try to issue id
    ///
    /// If issue id successfully, query osinfo additionally(and return IntermediateOS)
    async fn issue_id(&mut self) -> Result<Option<HighOS>, packet::Error> {
        if self.packet.get_handshake_uid()? == 0 {
            let id = self.id_counter.clone();
            *self.id_counter += 1;
            self.packet.issue_id(id).await?;

            let os_info = self.packet.os_query().await?;

            let grub_info: Vec<GrubSec> = self
                .packet
                .grub_query()
                .await?
                .into_iter()
                .map(|info| info.grub_sec)
                .collect();

            let os = HighOS {
                id: self.packet.get_handshake_uid()?,
                display_name: os_info.display_name,
                unknown_edge: grub_info,
            };
            self.ioss.push(os.clone());
            Ok(Some(os))
        } else {
            Ok(None)
        }
    }
    /// Returns the trace(a series of BootMethod) to closest os with unknown edge
    fn get_closest_trace(&mut self) -> Result<(HighOS, Vec<BootMethod>), packet::Error> {
        let current_os = LowOS {
            id: self.packet.get_handshake_uid()?,
        };
        let current_node = self.graph.find_node(&OSState::Up(current_os)).unwrap();

        let dijkstra = self.graph.dijkstra(&current_node);

        // get the index of closest os
        let unknown_os: Vec<LowOS> = self
            .unknown_os
            .iter()
            .map(|ios| ios.clone().into_low())
            .collect();
        let unknown_os_distance: Vec<usize> = unknown_os
            .into_iter()
            .map(|os| {
                let node = self.graph.find_node(&OSState::Up(os)).unwrap();
                dijkstra.to(&node).unwrap()
            })
            .collect();
        let min_index = unknown_os_distance
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(index, _)| index)
            .unwrap();

        let closest_node = self
            .graph
            .find_node(&OSState::Up(self.unknown_os[min_index].into_low()))
            .unwrap();
        let trace = dijkstra.trace(&closest_node).unwrap();
        let trace = trace.iter().map(|x| (*x).clone()).collect();

        let os = self.unknown_os.swap_remove(min_index);

        Ok((os, trace))
    }
    fn into_inner(mut self) -> (BootGraph, Packet<'a>) {
        let mut map = BTreeMap::new();

        for ios in self.ioss {
            map.insert(
                ios.id.clone(),
                OS {
                    id: ios.id,
                    display_name: ios.display_name,
                },
            );
        }

        let graph = self.graph.transform_node(|org| match org {
            OSState::Down => OSState::Down,
            OSState::Up(os) => OSState::Up(map.remove(&os.id).unwrap()),
        });
        (BootGraph { graph }, self.packet)
    }
    fn is_finish(&self) -> bool {
        self.unknown_os.is_empty()
    }
    pub async fn tick(&mut self) -> Result<(), packet::Error> {
        while !self.is_finish() {
            let (mut ios, trace) = self.get_closest_trace()?;

            for method in &trace {
                method.execute(&mut self.packet).await?;
            }

            let from = ios.into_low();
            let grub_sec = ios.unknown_edge.pop().unwrap();
            let method = BootMethod::Grub(grub_sec);

            method.execute(&mut self.packet).await?;

            let dist = match self.issue_id().await? {
                Some(ios) => ios.into_low(),
                None => LowOS {
                    id: self.packet.get_handshake_uid()?,
                },
            };

            let from = self.graph.find_node(&OSState::Up(from)).unwrap();
            let dist = self.graph.find_node(&OSState::Up(dist)).unwrap();

            self.graph.connect(from, dist, method);

            // put ios back if not fully emptied yet
            if !ios.unknown_edge.is_empty() {
                self.unknown_os.push(ios);
            }
        }
        todo!()
    }
}

struct BootGraph {
    graph: Graph<OSState<OS>, BootMethod>,
}
