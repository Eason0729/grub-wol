use crate::constant;
use serde::{Deserialize, Serialize};
// use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    Handshake(HandShake),
    Answer(Answer),
    Request(Request),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HandShake {
    pub mac_address: [u8; 6],
    pub uid: constant::ID,
    pub version: constant::APIVersionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Request {
    Reboot(BootInto),
    GrubQuery,
    Alive,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Answer {
    Reboot(BootInto),
    GrubQuery,
    IsAlive(constant::ID),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GrubData {
    pub list: Vec<GrubDescription>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GrubDescription {
    pub grub_sec: constant::Integer,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum BootInto {
    Down,
    OperatingSystem(constant::GrubId),
}
