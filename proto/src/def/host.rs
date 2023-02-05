use crate::constant;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    Handshake(Handshake),
    Reboot,
    InitId,
    ShutDown,
    GrubQuery(Vec<GrubInfo>),
    Ping(constant::ID),
    OSQuery(OSQuery),
}

pub type GrubQuery=Vec<GrubInfo>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Handshake {
    pub ident: constant::ProtoIdentType,
    pub mac_address: [u8; 6],
    pub uid: constant::ID,
    pub version: constant::APIVersionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GrubInfo {
    pub grub_sec: constant::GrubId,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OSQuery {
    pub display_name: String,
}
