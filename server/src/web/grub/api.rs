use serde::Serialize;
use super::bootgraph;

// mac_address in PATH: 0x{hex}

// boot into a os
// UPDATE /op/up/:mac_address/:os

// shutdown a machine
// UPDATE /op/down/:mac_address

// get a list of machine (response) 
// GET /machines
#[derive(Serialize)]
pub struct MachineList<'a>{
    machines:Vec<MachineInfo<'a>>
}

// get a list of machine (response) 
// GET /machine/:mac_address
#[derive(Serialize)]
pub struct MachineInfo<'a>{
    is_inited:bool,
    mac_address:&'a [u8;6],
    current_os:Option<bootgraph::OSId>
}

// get detailed info of os (response) 
// GET /os/:os_id
#[derive(Serialize)]
pub struct OsInfo<'a>{
    display_name:&'a str,
    id:bootgraph::OSId
}

// get a list of os (response) 
// GET /oss/:mac_address
#[derive(Serialize)]
pub struct OsList<'a>{
    pub oss:Vec<OsInfo<'a>>
}
