use crate::constant;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    HandShake(HandShake),
    Reboot,
    InitId,
    ShutDown,
    GrubQuery(Vec<GrubInfo>),
    Ping(constant::ID),
    OSQuery(OSInfo),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HandShake {
    pub ident: constant::ProtoIdentType,
    pub mac_address: [u8; 6],
    pub uid: constant::ID,
    pub version: constant::APIVersionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GrubInfo {
    pub grub_sec: constant::Integer,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OSInfo {
    pub display_name: String,
}
