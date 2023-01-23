use std::mem;
use std::{pin::Pin, sync::Arc};

use super::bootgraph;
use super::machine::{Error, Machine, Server};
use async_trait::async_trait;
use futures_lite::Future;
use log::warn;
use proto::prelude::ID;
use serde::Serialize;
use web;

#[async_trait]
trait Convert<K>
where
    K: Serialize,
{
    async fn convert(self) -> Result<Vec<u8>, Error>;
}

pub struct OsListAdaptor<'a> {
    pub(super) machine: Option<Arc<Machine<'a>>>,
}
#[async_trait]
impl<'a> Convert<web::OsList<'a>> for OsListAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        match self.machine {
            Some(machine) => {
                let oss = machine
                    .boot_graph
                    .list_os()
                    .map(|os| web::OsInfoInner {
                        display_name: &os.display_name,
                        id: os.id,
                    })
                    .collect();
                Ok(bincode::serialize(&web::OsList { oss }).unwrap())
            }
            None => Ok(bincode::serialize(&web::OsList { oss: Vec::new() }).unwrap()),
        }
    }
}

pub struct MachineInfoAdaptor<'a> {
    pub(super) machine: Option<Arc<Machine<'a>>>,
}

#[async_trait]
impl<'a> Convert<web::MachineInfo<'a>> for MachineInfoAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        match self.machine {
            Some(machine) => {
                let current_os = match machine.current_os().await? {
                    Some(os) => Some(os),
                    None => None,
                };

                Ok(bincode::serialize(&Some(web::MachineInfoInner {
                    mac_address: &machine.mac_address,
                    state: match current_os {
                        Some(os) => web::MachineState::Up(os),
                        None => web::MachineState::Down,
                    },
                }))
                .unwrap())
            }
            None => Ok(bincode::serialize::<Option<web::MachineInfoInner>>(&None).unwrap()),
        }
    }
}

pub struct MachineListAdaptor<'a, 'b> {
    pub(super) server: &'a Server<'b>,
}

#[async_trait]
impl<'a, 'b> Convert<web::MachineList<'a>> for MachineListAdaptor<'a, 'b> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        let mut machines = Vec::new();
        let server = self.server;

        let machines_src = server.machines.lock().await;
        for (mac_address, machine) in machines_src.iter() {
            let current_os = machine.current_os().await?.map(|os| os);
            machines.push(web::MachineInfoInner {
                state: match current_os {
                    Some(os) => web::MachineState::Up(os),
                    None => web::MachineState::Down,
                },
                mac_address,
            });
        }

        let unknown_src = server.unknown_packet.lock().await;
        let unknown_mac: Vec<[u8; 6]> = unknown_src
            .iter()
            .map(|p| p.get_mac())
            .collect();
        unknown_mac.iter().for_each(|mac_address| {
            machines.push(web::MachineInfoInner {
                mac_address,
                state: web::MachineState::Uninited,
            });
        });

        Ok(bincode::serialize(&web::MachineList { machines }).unwrap())
    }
}

pub struct BootAdaptor<'a> {
    pub(super) os: web::OSState,
    pub(super) machine: Option<Arc<Machine<'a>>>,
}

#[async_trait]
impl<'a> Convert<web::BootRes> for BootAdaptor<'a> {
    async fn convert(self) -> Result<Vec<u8>, Error> {
        let os = match self.os {
            web::OSState::Down => bootgraph::OSState::Down,
            web::OSState::Up(x) => bootgraph::OSState::Up(x),
        };

        if self.machine.is_none(){
            return Ok(bincode::serialize(&web::BootRes::NotFound).unwrap());
        }
        let machine=self.machine.unwrap();
        let mut packet = machine.packet.lock().await;
        let mut out_packet=None;
        mem::swap(&mut out_packet,&mut *packet);
        drop(packet);

        match out_packet{
            Some(mut packet)=>{
                let mac = packet.get_mac(); 
                let raw = match machine.boot_graph.boot_into(os, &mut packet, mac).await {
                    Ok(_) => web::BootRes::Success,
                    Err(e) => {
                        warn!("{}", e);
                        web::BootRes::Fail
                    }
                };
                Ok(bincode::serialize(&raw).unwrap())
            },
            None=>{
                Ok(bincode::serialize(&web::BootRes::NotFound).unwrap())
            }
        }
    }
}
// new machine
