use std::marker::PhantomData;
use std::{fs, io, net, path};

use crate::packet::Packets;

use super::graph::Graph;

use proto::prelude as protocal;
use proto::prelude::{Answer, GrubData, GrubDescription, Packet, Request};
use serde::{Deserialize, Serialize};
use std::collections::*;

type NodeId = usize;
type MacAddress = [u8; 6];

#[derive(Serialize, Deserialize)]
struct GrubAction {
    grub_sec: usize,
}

#[derive(Serialize, Deserialize)]
struct Server {
    machines: BTreeMap<MacAddress, Machine>,
    #[serde(skip)]
    packets: Packets,
}

impl Server {
    fn connect(&mut self) {
        // connect to one machine instance by mac address
    }
}

#[derive(Serialize, Deserialize)]
struct Machine {
    boot_graph: Graph<OS, usize>,
}

impl Machine {
    fn new_instance(&mut self) {}
}

#[derive(Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
struct OS {
    display_name: String,
    id: protocal::ID,
}

struct MachineInstance {
    id: protocal::ID,
}
