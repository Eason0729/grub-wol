use serde::{Deserialize, Serialize};

use crate::constant;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    HandShake(HandShake),
    Reboot(constant::ID),
    InitId(constant::ID),
    ShutDown,
    GrubQuery,
    IsAlive
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HandShake {
    pub version: constant::APIVersionType,
}
