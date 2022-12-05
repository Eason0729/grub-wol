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
    mac_address: [u8; 6],
    uid: constant::ID,
    version: constant::APIVersionType,
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
    list: Vec<GrubDescription>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GrubDescription {
    grub_sec: constant::Integer,
    display_name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum BootInto {
    Down,
    OperatingSystem(constant::GrubId),
}
