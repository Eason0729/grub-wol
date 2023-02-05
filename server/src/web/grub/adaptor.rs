use std::borrow::Cow;
use std::mem;
use std::sync::Arc;

use super::machine::{Error, Machine, Server};
use super::{api, bootgraph};
use async_trait::async_trait;
use log::warn;
use monostate::MustBeStr::MustBeStr;
use serde::Serialize;

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
impl<'a> Convert<api::OsList<'a>> for OsListAdaptor {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        match self.machine {
            Some(machine) => {
                let oss = machine
                    .boot_graph
                    .list_os()
                    .map(|os| api::OsInfoInner {
                        display_name: Cow::Borrowed(&os.display_name),
                        id: os.id,
                    })
                    .collect();
                Ok(serde_json::to_vec(&api::OsList { oss }).unwrap())
            }
            None => Ok(serde_json::to_vec(&api::OsList { oss: Vec::new() }).unwrap()),
        }
    }
}

pub struct MachineInfoAdaptor {
    pub(super) machine: Option<Arc<Machine>>,
}

#[async_trait]
impl<'a> Convert<api::MachineInfo<'a>> for MachineInfoAdaptor {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        match self.machine {
            Some(machine) => {
                let current_os = match machine.current_os().await? {
                    Some(os) => Some(os),
                    None => None,
                };
                let display_name = &*machine.display_name.lock().await.to_owned();
                Ok(serde_json::to_vec(&Some(api::MachineInfoInner {
                    display_name: Some(Cow::Borrowed(&display_name)),
                    mac_address: Cow::Borrowed(&machine.mac_address),
                    state: match current_os {
                        Some(os) => api::MachineState::Up { id: os },
                        None => api::MachineState::Down { kind: MustBeStr },
                    },
                }))
                .unwrap())
            }
            None => Ok(serde_json::to_vec::<Option<api::MachineInfoInner>>(&None).unwrap()),
        }
    }
}

pub struct MachineListAdaptor<'a> {
    pub(super) server: &'a Server,
}

#[async_trait]
impl<'a> Convert<api::MachineList<'a>> for MachineListAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        let mut machines = Vec::new();
        let server = self.server;

        let machines_src = server.machines.lock().await;
        for (mac_address, machine) in machines_src.iter() {
            let current_os = machine.current_os().await?.map(|os| os);
            let display_name = machine.display_name.lock().await.to_owned();
            machines.push(api::MachineInfoInner {
                display_name: Some(Cow::Owned(display_name)),
                state: match current_os {
                    Some(os) => api::MachineState::Up { id: os },
                    None => api::MachineState::Down { kind: MustBeStr },
                },
                mac_address: Cow::Borrowed(mac_address),
            });
        }

        let unknown_src = server.unknown_packet.lock().await;
        let unknown_mac: Vec<[u8; 6]> = unknown_src
            .iter()
            .map(|p| p.get_mac_address().clone())
            .collect();
        unknown_mac.iter().for_each(|mac_address| {
            machines.push(api::MachineInfoInner {
                display_name: None,
                mac_address: Cow::Borrowed(mac_address),
                state: api::MachineState::Uninited { kind: MustBeStr },
            });
        });

        Ok(serde_json::to_vec(&api::MachineList { machines }).unwrap())
    }
}

pub struct BootAdaptor {
    pub(super) os: api::OsStatus,
    pub(super) machine: Option<Arc<Machine>>,
}

#[async_trait]
impl Convert<api::BootRes> for BootAdaptor {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        let os = match self.os {
            api::OsStatus::Down { kind: _ } => bootgraph::OsStatus::Down,
            api::OsStatus::Up { id } => bootgraph::OsStatus::Up(id),
        };

        if self.machine.is_none() {
            return Ok(serde_json::to_vec(&api::BootRes::NotFound).unwrap());
        }
        let machine = self.machine.unwrap();
        let mut packet = machine.packet.lock().await;
        let mut out_packet = None;
        mem::swap(&mut out_packet, &mut *packet);
        drop(packet);

        match out_packet {
            Some(mut packet) => {
                let raw = match machine.boot_graph.boot_into(os, &mut packet).await {
                    Ok(_) => api::BootRes::Success,
                    Err(e) => {
                        warn!("{}", e);
                        api::BootRes::Fail
                    }
                };
                Ok(serde_json::to_vec(&raw).unwrap())
            }
            None => Ok(serde_json::to_vec(&api::BootRes::NotFound).unwrap()),
        }
    }
}

pub struct NewMachineAdaptor<'a> {
    pub(super) display_name: String,
    pub(super) mac_address: [u8; 6],
    pub(super) server: &'a Server,
}

#[async_trait]
impl<'a> Convert<api::NewMachineRes> for NewMachineAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        // TODO: notice the client after instead of creating a long running http request
        Ok(serde_json::to_vec(&match self
            .server
            .new_machine(self.mac_address, self.display_name)
            .await
        {
            Ok(x) => {
                if x {
                    api::NewMachineRes::Success
                } else {
                    api::NewMachineRes::NotFound
                }
            }
            Err(_) => api::NewMachineRes::Fail,
        })
        .unwrap())
    }
}
// new machine
