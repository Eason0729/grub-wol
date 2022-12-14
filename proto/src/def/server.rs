use nanoserde::{DeBin,SerBin};

use crate::constant;

#[derive(DeBin,SerBin, Debug, PartialEq)]
pub enum Packet {
    HandShake(HandShake),
    Reboot(constant::ID),
    InitId(constant::ID),
    ShutDown,
    GrubQuery,
    IsAlive
}

#[derive(DeBin,SerBin, Debug, PartialEq)]
pub struct HandShake {
    pub version: constant::APIVersionType,
}
