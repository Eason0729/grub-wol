use std::marker::PhantomData;
use std::{fs, io, net, path};

use crate::event;

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
struct Machines {}

impl Machines {
    fn connect(&mut self) {
        // connect to one machine instance by mac address
    }
}
