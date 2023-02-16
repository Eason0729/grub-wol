use std::collections::HashMap;

use indexmap::IndexMap;
use proto::prelude::{GrubId, ID};
use serde::{Deserialize, Serialize};

use crate::grub::packet::{self, TcpPacket};

use super::graph::{Graph, Node};

#[derive(Hash, Eq, PartialEq, Clone, Deserialize, Serialize, Debug)]
pub enum BootMethod {
    WOL,
    Grub(GrubId),
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
                packet.read_reboot().await?;
                packet.wait_reconnect().await?;
            }
            BootMethod::Shutdown => {
                log::trace!("shuting down host {:?}", packet.get_mac_address());
                packet.write_shutdown().await?;
                packet.read_shutdown().await?;
            }
        };
        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Deserialize, Serialize, Debug)]
pub enum OsStatus {
    Down,
    Up(ID),
}

struct Helper {
    packet: TcpPacket,
    unknowns: HashMap<ID, Vec<BootMethod>>,
    offline: Node,
    graph: BootGraph,
}

impl Helper {
    fn new(packet: TcpPacket) -> Self {
        log::debug!("bootgraph helper is constructed");
        let mut graph = BootGraph::default();
        let offline = graph.graph.add_node(OsStatus::Down);
        Self {
            packet,
            unknowns: HashMap::new(),
            offline,
            graph,
        }
    }
    async fn set_uid(&mut self) -> Result<ID, Error> {
        let uid = self.graph.id_counter;
        self.graph.id_counter += 1;
        self.packet.set_uid(uid)?;
        self.packet.write_init_id(uid).await?;
        self.packet.read_init_id().await?;
        log::trace!("os id of ${} was issue for the client",uid);
        Ok(uid)
    }
    async fn init_os(&mut self) -> Result<(), Error> {
        let uid = self.set_uid().await?;
        // perform grub query
        self.packet.write_grub_query().await?;
        let grub_list = self.packet.read_grub_query().await?;
        if !grub_list.is_empty(){
            self.unknowns.insert(
                uid,
                grub_list.into_iter().map(|info| BootMethod::Grub(info.grub_sec)).collect(),
            );
        }
        // perform os query
        self.packet.write_os_query().await?;
        let os_query = self.packet.read_os_query().await?;
        let os_info = OsInfo::from_query(os_query);
        // build graph
        log::debug!("inited os {}",os_info.display_name);
        self.graph.os.insert(uid, os_info);
        let node = self.graph.graph.add_node(OsStatus::Up(uid));
        self.graph
            .graph
            .connect(node, self.offline, BootMethod::Shutdown);
        Ok(())
    }
    async fn get_node(&self) -> Result<Node, Error> {
        let uid = self.packet.get_uid().await?;
        Ok(self.graph.graph.find_node(&OsStatus::Up(uid)).unwrap())
    }
    async fn trace_unknown(&mut self) -> Result<BootMethod, Error> {
        let uid = self.packet.get_uid().await?;
        let list=self.unknowns.get_mut(&uid).ok_or(Error::BadGraph).unwrap();
        let path=list.pop().ok_or(Error::BadGraph).unwrap();
        if list.is_empty(){
            self.unknowns.remove(&uid).unwrap();
        }
        path.execute(&mut self.packet).await?;
        log::trace!("randomly picked a unknown edge to discover");
        Ok(path)
    }
    async fn reset(&mut self) -> Result<(), Error> {
        self.packet.write_shutdown().await?;
        self.packet.read_shutdown().await?;
        self.packet.wol_reconnect().await?;
        Ok(())
    }
    async fn trace_closest_with_unknown(&mut self) -> Result<(), Error> {
        let from_node = self.get_node().await?;
        let dijkstra = self.graph.graph.dijkstra(&from_node);

        let unsafe_node = unsafe { Node::new(0) };
        let closest_node = self
            .unknowns
            .iter()
            .map(|(uid, _)| {
                let node = self.graph.graph.find_node(&OsStatus::Up(*uid)).unwrap();
                let distance = dijkstra.to(&node).unwrap_or(usize::MAX);
                (node, distance)
            })
            .fold(
                (unsafe_node, usize::MAX),
                |(acc_node, acc_distance), (node, distance)| {
                    if acc_distance > distance {
                        (node, distance)
                    } else {
                        (acc_node, acc_distance)
                    }
                },
            )
            .0;
        let trace = dijkstra
            .trace(&closest_node)
            .unwrap();
        for pat in trace{
            pat.execute(&mut self.packet).await?;
        }
        log::trace!("booted to closest node with unknown edge");
        Ok(())
    }
    async fn construct_wol_edge(&mut self)-> Result<(), Error> {
        let offline = self.offline;
        let node = self.get_node().await?;
        self.graph.graph.connect(offline, node, BootMethod::WOL);
        Ok(())
    }
    async fn is_os_inited(&self) -> Result<bool, Error> {
        Ok(self.packet.get_uid().await? != 0)
    }
    fn is_finish(&self) -> bool {
        self.unknowns.is_empty()
    }
    fn finialize(self)->(BootGraph,TcpPacket){
        (self.graph,self.packet)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BootGraph {
    graph: Graph<OsStatus, BootMethod>,
    os: IndexMap<ID, OsInfo>,
    id_counter: ID,
}

impl Default for BootGraph {
    fn default() -> Self {
        Self { graph: Default::default(), os: Default::default(), id_counter: 1 }
    }
}

impl BootGraph {
    pub async fn new(packet: TcpPacket) -> Result<(Self,TcpPacket), Error> {
        let mut helper = Helper::new(packet);
        helper.reset().await?;
        helper.init_os().await?;
        helper.construct_wol_edge().await?;

        while !helper.is_finish() {
            if !helper.is_os_inited().await? {
                helper.init_os().await?;
            }
            // boot to a node with unknown edge
            helper.trace_closest_with_unknown().await?;
            let from_node=helper.get_node().await?;
            // boot to any unknown edge
            let unknown_edge=helper.trace_unknown().await.unwrap();
            if !helper.is_os_inited().await? {
                helper.init_os().await?;
            }
            let to_node=helper.get_node().await?;
            helper.graph.graph.connect(from_node, to_node, unknown_edge);
        }
        Ok(helper.finialize())
    }
    pub async fn current_os(&self, packet: &TcpPacket) -> Result<OsStatus, Error> {
        match packet.get_uid().await {
            Ok(x) => {
                Ok(OsStatus::Up(x))
            }
            Err(e) => {
                match e {
                    packet::Error::ClientOffline => Ok(OsStatus::Down),
                    _ => Err(e.into()),
                }
            }
        }
    }
    pub fn list_os(&self) -> impl Iterator<Item = (&ID,&OsInfo)> {
        self.os.iter()
    }
    pub fn find_os(&self, os: ID) -> Option<&OsInfo> {
        self.os.get(&os)
    }
    pub async fn boot(&self,os:OsStatus,packet:&mut TcpPacket)->Result<(),Error>{
        let from_os=match packet.get_uid().await{
            Ok(x) => Ok(OsStatus::Up(x)),
            Err(err) => match err{
                packet::Error::ClientOffline => Ok(OsStatus::Down),
                packet::Error::Timeout |  packet::Error::Conn(_) => Err(err).into(),
            },
        }?;
        let from_node=self.graph.find_node(&from_os).ok_or(Error::BadGraph)?;
        let to_node=self.graph.find_node(&os).ok_or(Error::BadGraph)?;

        let dijkstra=self.graph.dijkstra(&from_node);
        let trace=dijkstra.trace(&to_node).ok_or(Error::BadGraph)?;
        for pat in trace{
            pat.execute(packet).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct OsInfo {
    pub display_name: String,
}

impl OsInfo {
    fn from_query(query: proto::prelude::host::OsQuery) -> Self {
        Self {
            display_name: query.display_name,
        }
    }
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
