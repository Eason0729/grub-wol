use crate::{
    machine::graph::Dijkstra,
    packet::{self, Packet, Packets},
};

use super::graph::Graph;

use proto::prelude as protocal;
use serde::{Deserialize, Serialize};
use std::collections::*;

#[derive(Clone, Ord, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum OS {
    Down,
    Up(OSInfo),
}

impl OS {
    pub fn from_info(info: protocal::host::OsInfo, id: protocal::ID) -> Self {
        let info = OSInfo {
            display_name: info.display_name,
            id,
        };
        OS::Up(info)
    }
}

#[derive(Clone, Ord, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum BootMethod {
    WOL,
    Grub(protocal::Integer),
    Down,
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
