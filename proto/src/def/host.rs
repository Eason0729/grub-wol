use crate::constant;
use serde::{Deserialize, Serialize};

pub type GrubQuery=Vec<GrubInfo>;
pub type Ping=constant::ID;
pub type Reboot=();
pub type InitId=();
pub type Shutdown=();
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    Handshake(Handshake),
    Reboot,
    InitId,
    Shutdown,
    GrubQuery(GrubQuery),
    Ping(Ping),
    OsQuery(OsQuery),
}


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
pub struct OsQuery {
    pub display_name: String,
}
