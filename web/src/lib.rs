/// file for api response
use proto::prelude::ID;
use serde::{Serialize, Deserialize};

// boot into a os (request)
// POST /api/UPDATE/op/up
// cts
#[derive(Deserialize,Serialize)]
pub struct BootReq{
    mac_address: [u8; 6],
    os:ID
}
// stc: bool

// shutdown a machine
// POST /api/UPDATE/op/down
// cts
#[derive(Deserialize,Serialize)]
pub struct ShutdownReq{
    mac_address: [u8; 6]
}
// stc: bool

// get a list of machine
// POST /api/GET/machines
// cts: no payload
// stc
#[derive(Deserialize,Serialize)]
pub struct MachineList {
    pub machines: Vec<MachineInfo>,
}

// get detailed info of a machine
// POST /api/machine
// cts
#[derive(Deserialize,Serialize)]
pub struct MachineInfoReq{
    mac_address: [u8; 6]
}
// stc
// return type is wrapped in option
#[derive(Deserialize,Serialize)]
pub struct MachineInfo {
    pub mac_address: [u8; 6],
    pub state: MachineState,
}

// get a list of os
// POST /api/GET/oss
// :mac_address
#[derive(Deserialize,Serialize)]
pub struct OsList {
    pub oss: Vec<OsInfo>,
}

#[derive(Deserialize,Serialize)]
pub enum MachineState {
    Down,
    Uninited,
    Up(ID),
}

#[derive(Deserialize,Serialize)]
pub struct OsInfo {
    pub display_name: String,
    pub id: ID,
}