use super::bootgraph;
use serde::Serialize;

// mac_address in PATH: 0x{hex}

// boot into a os
// UPDATE /op/up/:mac_address/:os_id

// shutdown a machine
// UPDATE /op/down/:mac_address

// get a list of machine (response)
// GET /machines
#[derive(Serialize)]
pub struct MachineList<'a> {
    pub machines: Vec<MachineInfo<'a>>,
}

// get detailed info of a machine (response)
// GET /machine/:mac_address
// return type is wrapped in option
#[derive(Serialize)]
pub struct MachineInfo<'a> {
    pub mac_address: &'a [u8; 6],
    pub state: MachineState,
}

#[derive(Serialize)]
pub enum MachineState {
    Down,
    Uninited,
    Up(bootgraph::OSId),
}

#[derive(Serialize)]
pub struct OsInfo<'a> {
    pub display_name: &'a str,
    pub id: bootgraph::OSId,
}

// get a list of os (response)
// GET /oss/:mac_address
#[derive(Serialize)]
pub struct OsList<'a> {
    pub oss: Vec<OsInfo<'a>>,
}
