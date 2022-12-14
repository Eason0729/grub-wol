use crate::constant;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    HandShake(HandShake),
    GrubQuery(Vec<GrubDescription>),
    IsAlive(constant::ID),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HandShake {
    pub mac_address: [u8; 6],
    pub uid: constant::ID,
    pub version: constant::APIVersionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GrubDescription {
    pub grub_sec: constant::Integer,
    pub display_name: String,
}
