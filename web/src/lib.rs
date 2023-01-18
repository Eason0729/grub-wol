/// file for api response
use proto::prelude::ID;
use serde::{Deserialize, Serialize};

// boot into a os (request)
// POST /api/op/up
// cts
#[derive(Deserialize, Serialize)]
pub struct BootReq {
    pub mac_address: [u8; 6],
    pub os: ID,
}
// stc: bool

// shutdown a machine
// POST /api/op/down
// cts
#[derive(Deserialize, Serialize)]
pub struct ShutdownReq {
    pub mac_address: [u8; 6],
}
// stc: bool

// get a list of machine
// POST /api/get/machines
// cts: no payload
// stc
#[derive(Deserialize, Serialize)]
pub struct MachineList {
    pub machines: Vec<MachineInfo>,
}

// get detailed info of a machine
// POST /api/get/machine
// cts
#[derive(Deserialize, Serialize)]
pub struct MachineInfoReq {
    pub mac_address: [u8; 6],
}
// stc
// return type is wrapped in option
#[derive(Deserialize, Serialize)]
pub struct MachineInfo {
    pub mac_address: [u8; 6],
    pub state: MachineState,
}

// get a list of os
// POST /api/get/oss
// :mac_address
#[derive(Deserialize, Serialize)]
pub struct OsList {
    pub oss: Vec<OsInfo>,
}

#[derive(Deserialize, Serialize)]
pub enum MachineState {
    Down,
    Uninited,
    Up(ID),
}

#[derive(Deserialize, Serialize)]
pub struct OsInfo {
    pub display_name: String,
    pub id: ID,
}
