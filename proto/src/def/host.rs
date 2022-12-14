use crate::constant;
use nanoserde::{DeBin,SerBin};

#[derive(DeBin,SerBin, Debug, PartialEq)]
pub enum Packet {
    HandShake(HandShake),
    GrubQuery(Vec<GrubDescription>),
    IsAlive(constant::ID),
}

#[derive(DeBin,SerBin, Debug, PartialEq)]
pub struct HandShake {
    pub mac_address: [u8; 6],
    pub uid: constant::ID,
    pub version: constant::APIVersionType,
}

#[derive(DeBin,SerBin, Debug, PartialEq)]
pub struct GrubDescription {
    pub grub_sec: constant::Integer,
    pub display_name: String,
}
