use serde::{Deserialize, Serialize};

use crate::constant;

pub type Reboot=constant::GrubId;
pub type InitId=constant::ID;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    Handshake(Handshake),
    Reboot(constant::GrubId), // rpc: execute grub reboot
    InitId(constant::ID),
    Shutdown,  // rpc: execute grub reboot
    GrubQuery, // query: query available grub path
    Ping,
    OsQuery, // query: query current os info
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Handshake {
    pub ident: constant::ProtoIdentType,
    pub version: constant::APIVersionType,
}
