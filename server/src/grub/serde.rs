use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
    sync::Arc,
};

use super::{
    bootgraph::BootGraph,
    machine::{Machine, Server},
};
use ::serde::{Deserialize, Serialize};
use async_std::{
    fs::File,
    io::{ReadExt, WriteExt},
    sync::Mutex,
};
use async_trait::async_trait;
use indexmap::IndexMap;
use proto::prelude::SERVER_PORT;

#[async_trait]
pub trait AsyncState<O>
where
    Self: for<'a> Deserialize<'a> + Serialize + Default,
    O: Sync,
{
    async fn serde(machine: &O) -> Self;
    fn deserde(self) -> O;
    async fn load(path: &Path) -> O {
        let save = if path.exists() && path.is_file() {
            let mut file = File::open(path).await.unwrap();

            let buf = &mut Vec::new();
            file.read_to_end(buf).await.unwrap();

            bincode::deserialize::<Self>(buf).unwrap()
        } else {
            Default::default()
        };
        save.deserde()
    }
    async fn save(src: &O, path: &Path) {
        let buf = bincode::serialize(&Self::serde(src).await).unwrap();

        log::trace!("Serialized save file");
        let mut file = File::open(path).await.unwrap();
        file.write_all(&buf).await.unwrap();
        log::info!("Saving Done");
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct MachineSave {
    display_name: String,
    mac_address: [u8; 6],
    boot_graph: BootGraph,
}

#[async_trait]
impl AsyncState<Machine> for MachineSave {
    async fn serde(machine: &Machine) -> MachineSave {
        MachineSave {
            display_name: (&*machine.display_name.lock().await).clone(),
            mac_address: machine.mac_address.clone(),
            boot_graph: machine.boot_graph.clone(),
        }
    }
    fn deserde(self) -> Machine {
        Machine {
            display_name: Mutex::new(self.display_name),
            mac_address: self.mac_address,
            boot_graph: self.boot_graph,
            packet: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct ServerSave {
    machines: IndexMap<[u8; 6], MachineSave>,
}

#[async_trait]
impl AsyncState<Server> for ServerSave {
    async fn serde(server: &Server) -> ServerSave {
        let mut machines = IndexMap::new();
        for (mac, machine) in &*(server.machines.lock().await) {
            machines.insert(mac.clone(), MachineSave::serde(&**machine).await);
        }
        let machines = machines.into();
        ServerSave { machines }
    }
    fn deserde(self) -> Server {
        let bind_host = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let socket = std::net::SocketAddr::new(bind_host, SERVER_PORT);

        let machines = self
            .machines
            .into_iter()
            .map(|(key, value)| (key, Arc::new(value.deserde())))
            .collect();
        Server {
            machines: Mutex::new(machines),
            packets: Default::default(),
            unknown_packet: Default::default(),
            socket,
        }
    }
}
