use serde::{Deserialize, Serialize};

use crate::constant;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet {
    HandShake(HandShake),
    Reboot(constant::ID),
    ShutDown,
    GrubQuery,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HandShake {
    pub version: constant::APIVersionType,
    pub id: constant::ID,
}
