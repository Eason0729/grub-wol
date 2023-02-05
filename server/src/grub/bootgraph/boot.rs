// TODO: fix error handling
use super::super::packet::{self, TcpPacket};

use super::graph::Graph;

use indexmap::IndexMap;
use proto::prelude as protocal;
use serde::{Deserialize, Serialize};

pub type GrubSec = protocal::GrubId;

trait IntoLow {
    type Low;
    fn into_low(&self) -> Self::Low;
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Os {
    pub id: protocal::ID,
    pub display_name: String,
}

#[derive(Hash, Eq, PartialEq, Clone, Deserialize, Serialize)]
struct LowOs {
    id: protocal::ID,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct HighOs {
    id: protocal::ID,
    display_name: String,
    unknown_edge: Vec<GrubSec>,
}

impl IntoLow for HighOs {
    type Low = LowOs;

    fn into_low(&self) -> Self::Low {
        LowOs { id: self.id }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub enum OsStatus<T> {
    Down,
    Up(T),
}

impl<T> OsStatus<T> {
    fn map<F, O>(self, f: F) -> OsStatus<O>
    where
        F: Fn(T) -> O,
    {
        match self {
            OsStatus::Down => OsStatus::Down,
            OsStatus::Up(x) => OsStatus::Up(f(x)),
        }
    }
}

impl<T> IntoLow for OsStatus<T>
where
    T: IntoLow,
{
    type Low = OsStatus<T::Low>;

    fn into_low(&self) -> Self::Low {
        match self {
            OsStatus::Down => OsStatus::Down,
            OsStatus::Up(x) => OsStatus::Up(x.into_low()),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub enum BootMethod {
    WOL,
    Grub(GrubSec),
    Shutdown,
}

impl BootMethod {
    pub async fn execute(&self, packet: &mut TcpPacket) -> Result<(), packet::Error> {
        match self {
            BootMethod::WOL => {
                log::trace!("waiting host {:?} to boot", packet.get_mac_address());
                packet.wol_reconnect().await?;
            }
            BootMethod::Grub(x) => {
                log::trace!(
                    "executing host {:?} to chain load other os",
                    packet.get_mac_address()
                );
                packet.write_reboot(*x).await?;
                packet.wait_reconnect().await?;
            }
            BootMethod::Shutdown => {
                log::trace!("shuting down host {:?}", packet.get_mac_address());
                packet.write_shutdown().await?;
            }
        };
        Ok(())
    }
}

pub struct IntBootGraph {
    graph: Graph<OsStatus<LowOs>, BootMethod>,
    packet: TcpPacket,
    unknown_os: Vec<HighOs>,
    ioss: Vec<HighOs>,
    id_counter: protocal::ID,
    mac_address: [u8; 6],
}

impl IntBootGraph {
    pub async fn new(packet: TcpPacket, id_counter: protocal::ID) -> Result<IntBootGraph, Error> {
        let mac_address = packet.get_mac_address().clone();
        let mut self_ = IntBootGraph {
            graph: Graph::new(),
            packet,
            unknown_os: vec![],
            id_counter,
            ioss: vec![],
            mac_address,
        };

        // reboot to ensure correct first-boot os
        log::debug!("initing first-boot os");
        self_.packet.write_shutdown().await?;
        self_.packet.read_shutdown().await?;
        log::trace!("signal down");
        self_.packet.wol_reconnect().await?;

        // construct shutdown->first-boot-os on boot_graph
        let shutdown_node = self_.graph.add_node(OsStatus::Down);
        let fboot_os = self_
            .issue_id()
            .await?
            .ok_or(Error::UndefinedClientBehavior)?;
        let fboot_node = self_
            .graph
            .add_node(OsStatus::Up(fboot_os.clone()).into_low());

        self_
            .graph
            .connect(shutdown_node, fboot_node, BootMethod::WOL);

        // add fboot os to unknown
        self_.unknown_os.push(fboot_os);

        Ok(self_)
    }
    /// try to issue id
    ///
    /// If issue id successfully, query osinfo additionally(and return IntermediateOs)
    async fn issue_id(&mut self) -> Result<Option<HighOs>, Error> {
        if self.packet.get_uid().await? == 0 {
            let id = self.id_counter.clone();
            self.id_counter += 1;
            log::debug!("initing new os with id {}", id);

            self.packet.write_initid(id).await?;
            self.packet.read_initid().await?;
            self.packet.set_uid(id)?;
            log::trace!("set id {}", id);

            self.packet.write_osquery().await?;
            let os_info = self.packet.read_osquery().await?;
            log::trace!("get os_info of {:?}", os_info);

            self.packet.write_grubquery().await?;
            let grub_info: Vec<GrubSec> = self
                .packet
                .read_grubquery()
                .await?
                .into_iter()
                .map(|info| info.grub_sec)
                .collect();
            log::trace!("get grub_info of {:?}", grub_info);

            let os = HighOs {
                id: self.packet.get_uid().await?,
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
    async fn get_closest_trace(&mut self) -> Result<(HighOs, Vec<BootMethod>), Error> {
        let current_os = LowOs {
            id: self.packet.get_uid().await?,
        };
        let current_node = self
            .graph
            .find_node(&OsStatus::Up(current_os))
            .ok_or(Error::BadGraph)?;

        let dijkstra = self.graph.dijkstra(&current_node);

        // get the index of closest os
        let unknown_os: Vec<LowOs> = self
            .unknown_os
            .iter()
            .map(|ios| ios.clone().into_low())
            .collect();
        let unknown_os_distance: Vec<usize> = unknown_os
            .into_iter()
            .map(|os| {
                // TODO: here should return Err(Error:BadGraph)
                let node = self.graph.find_node(&OsStatus::Up(os)).unwrap();
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
            .find_node(&OsStatus::Up(self.unknown_os[min_index].into_low()))
            .unwrap();
        let trace = dijkstra.trace(&closest_node).unwrap();
        let trace = trace.iter().map(|x| (*x).clone()).collect();

        let os = self.unknown_os.swap_remove(min_index);

        Ok((os, trace))
    }
    pub fn disassemble(self) -> (BootGraph, TcpPacket, protocal::ID) {
        let mut map = IndexMap::new();

        for ios in self.ioss {
            map.insert(
                ios.into_low(),
                Os {
                    id: ios.id,
                    display_name: ios.display_name,
                },
            );
        }

        (
            BootGraph {
                graph: self.graph,
                os: map,
            },
            self.packet,
            self.id_counter,
        )
    }
    fn is_finish(&self) -> bool {
        self.unknown_os.is_empty()
    }
    pub async fn try_yield(&mut self) -> Result<(), Error> {
        let shutdown_node = self
            .graph
            .find_node(&OsStatus::Down)
            .ok_or(Error::BadGraph)?;
        while !self.is_finish() {
            let (mut ios, trace) = self.get_closest_trace().await?;

            for method in &trace {
                method.execute(&mut self.packet).await?;
            }

            let from = ios.into_low();
            let from = self
                .graph
                .find_node(&OsStatus::Up(from))
                .ok_or(Error::BadGraph)?;
            let grub_sec = ios.unknown_edge.pop().ok_or(Error::BadGraph)?;
            let method = BootMethod::Grub(grub_sec);

            method.execute(&mut self.packet).await?;

            let dist = match self.issue_id().await? {
                Some(ios) => {
                    let dist_os = ios.into_low();
                    let dist = self
                        .graph
                        .find_node(&OsStatus::Up(dist_os.clone()))
                        .unwrap();
                    self.graph.connect(shutdown_node, dist, BootMethod::WOL);
                    dist_os
                }
                None => LowOs {
                    id: self.packet.get_uid().await?,
                },
            };

            let dist = self.graph.find_node(&OsStatus::Up(dist)).unwrap();

            self.graph.connect(from, dist, method);

            // put ios back if not fully emptied yet
            if !ios.unknown_edge.is_empty() {
                self.unknown_os.push(ios);
            }
        }
        log::debug!("finish boot graph");
        Ok(())
    }
}

// TODO: use Mutex in BootGraph to change display_name on fly
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct BootGraph {
    graph: Graph<OsStatus<LowOs>, BootMethod>,
    os: IndexMap<LowOs, Os>,
}

pub type OsId = protocal::ID;

impl BootGraph {
    pub async fn current_os(&self, packet: &TcpPacket) -> Result<OsStatus<&Os>, Error> {
        match packet.get_uid().await {
            Ok(x) => {
                let os = self.os.get(&LowOs { id: x });
                match os {
                    Some(x) => Ok(OsStatus::Up(x)),
                    None => Err(Error::UndefinedClientBehavior),
                }
            }
            Err(e) => {
                if let packet::Error::ClientOffline = e {
                    Ok(OsStatus::Down)
                } else {
                    Err(e.into())
                }
            }
        }
    }
    pub fn list_os(&self) -> impl Iterator<Item = &Os> {
        self.os.iter().map(|(_, v)| v)
    }
    pub fn find_os(&self, os: OsId) -> Option<&Os> {
        self.os.get(&LowOs { id: os })
    }
    pub async fn boot_into(
        &self,
        os: OsStatus<protocal::ID>,
        packet: &mut TcpPacket,
    ) -> Result<(), Error> {
        let from = self.current_os(packet).await?.map(|x| LowOs { id: x.id });
        let from = self.graph.find_node(&from).ok_or(Error::BadGraph)?;

        let to = match os {
            OsStatus::Down => OsStatus::Down,
            OsStatus::Up(id) => OsStatus::Up(LowOs { id: id }),
        };
        let to = self.graph.find_node(&to).ok_or(Error::BadGraph)?;

        for method in self
            .graph
            .dijkstra(&from)
            .trace(&to)
            .ok_or(Error::BadGraph)?
        {
            method.execute(packet).await?;
        }

        Ok(())
    }
    // pub async fn off(&self, packet: &mut Packet<'_>, mac_address: [u8; 6]) -> Result<(), Error> {
    //     let from = self.current_os(packet)?.map(|x| LowOs { id: x.id });
    //     let from = self.graph.find_node(&from).ok_or(Error::BadGraph)?;

    //     let to = OsStatus::Down;
    //     let to = self.graph.find_node(&to).ok_or(Error::BadGraph)?;

    //     for method in self
    //         .graph
    //         .dijkstra(&from)
    //         .trace(&to)
    //         .ok_or(Error::BadGraph)?
    //     {
    //         method.execute(packet, &mac_address).await?;
    //     }

    //     Ok(())
    // }
}
#[derive(thiserror::Error, Debug)]

pub enum Error {
    #[error("Unknown Client Behavior")]
    UndefinedClientBehavior,
    #[error("maybe graph is badly created")]
    BadGraph,
    #[error("Packet Error")]
    PacketError(#[from] packet::Error),
}

// #[cfg(test)]
// mod test{
//     #[test]
//     fn new_intgraph(){

//     }
// }
