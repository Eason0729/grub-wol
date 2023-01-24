use std::borrow::Cow;
use std::mem;
use std::sync::Arc;

use super::bootgraph;
use super::machine::{Error, Machine, Server};
use async_trait::async_trait;
use log::warn;
use serde::Serialize;
use website;

#[async_trait]
pub trait Convert<K>
where
    K: Serialize,
{
    async fn convert(self) -> Result<Vec<u8>, Error>;
}

pub struct OsListAdaptor {
    pub(super) machine: Option<Arc<Machine>>,
}

#[async_trait]
impl<'a> Convert<website::OsList<'a>> for OsListAdaptor {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        match self.machine {
            Some(machine) => {
                let oss = machine
                    .boot_graph
                    .list_os()
                    .map(|os| website::OsInfoInner {
                        display_name: Cow::Borrowed(&os.display_name),
                        id: os.id,
                    })
                    .collect();
                Ok(bincode::serialize(&website::OsList { oss }).unwrap())
            }
            None => Ok(bincode::serialize(&website::OsList { oss: Vec::new() }).unwrap()),
        }
    }
}

pub struct MachineInfoAdaptor {
    pub(super) machine: Option<Arc<Machine>>,
}

#[async_trait]
impl<'a> Convert<website::MachineInfo<'a>> for MachineInfoAdaptor {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        match self.machine {
            Some(machine) => {
                let current_os = match machine.current_os().await? {
                    Some(os) => Some(os),
                    None => None,
                };

                Ok(bincode::serialize(&Some(website::MachineInfoInner {
                    mac_address: Cow::Borrowed(&machine.mac_address),
                    state: match current_os {
                        Some(os) => website::MachineState::Up(os),
                        None => website::MachineState::Down,
                    },
                }))
                .unwrap())
            }
            None => Ok(bincode::serialize::<Option<website::MachineInfoInner>>(&None).unwrap()),
        }
    }
}

pub struct MachineListAdaptor<'a> {
    pub(super) server: &'a Server,
}

#[async_trait]
impl<'a> Convert<website::MachineList<'a>> for MachineListAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        let mut machines = Vec::new();
        let server = self.server;

        let machines_src = server.machines.lock().await;
        for (mac_address, machine) in machines_src.iter() {
            let current_os = machine.current_os().await?.map(|os| os);
            machines.push(website::MachineInfoInner {
                state: match current_os {
                    Some(os) => website::MachineState::Up(os),
                    None => website::MachineState::Down,
                },
                mac_address: Cow::Borrowed(mac_address),
            });
        }

        let unknown_src = server.unknown_packet.lock().await;
        let unknown_mac: Vec<[u8; 6]> = unknown_src.iter().map(|p| p.get_mac()).collect();
        unknown_mac.iter().for_each(|mac_address| {
            machines.push(website::MachineInfoInner {
                mac_address: Cow::Borrowed(mac_address),
                state: website::MachineState::Uninited,
            });
        });

        Ok(bincode::serialize(&website::MachineList { machines }).unwrap())
    }
}

pub struct BootAdaptor {
    pub(super) os: website::OSState,
    pub(super) machine: Option<Arc<Machine>>,
}

#[async_trait]
impl Convert<website::BootRes> for BootAdaptor {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        let os = match self.os {
            website::OSState::Down => bootgraph::OSState::Down,
            website::OSState::Up(x) => bootgraph::OSState::Up(x),
        };

        if self.machine.is_none() {
            return Ok(bincode::serialize(&website::BootRes::NotFound).unwrap());
        }
        let machine = self.machine.unwrap();
        let mut packet = machine.packet.lock().await;
        let mut out_packet = None;
        mem::swap(&mut out_packet, &mut *packet);
        drop(packet);

        match out_packet {
            Some(mut packet) => {
                let raw = match machine.boot_graph.boot_into(os, &mut packet).await {
                    Ok(_) => website::BootRes::Success,
                    Err(e) => {
                        warn!("{}", e);
                        website::BootRes::Fail
                    }
                };
                Ok(bincode::serialize(&raw).unwrap())
            }
            None => Ok(bincode::serialize(&website::BootRes::NotFound).unwrap()),
        }
    }
}

pub struct NewMachineAdaptor<'a> {
    pub(super) display_name: String,
    pub(super) mac_address: [u8; 6],
    pub(super) server: &'a Server,
}

#[async_trait]
impl<'a> Convert<website::NewMachineRes> for NewMachineAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        Ok(bincode::serialize(&match self
            .server
            .new_machine(self.mac_address, self.display_name)
            .await
        {
            Ok(x) => {
                if x {
                    website::NewMachineRes::Success
                } else {
                    website::NewMachineRes::NotFound
                }
            }
            Err(_) => website::NewMachineRes::Fail,
        })
        .unwrap())
    }
}
// new machine
