use serde::{Deserialize, Serialize};

use crate::constant;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    Handshake(Handshake),
    Reboot(constant::GrubId), // rpc: execute grub reboot
    InitId(constant::ID),
    ShutDown,  // rpc: execute grub reboot
    GrubQuery, // query: query available grub path
    Ping,
    OsQuery, // query: query current os info
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Handshake {
    pub ident: constant::ProtoIdentType,
    pub version: constant::APIVersionType,
}
