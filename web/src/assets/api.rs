use std::borrow::Cow;
use monostate::MustBe;
/// file for api response
use proto::prelude::ID;
use serde::{Deserialize, Serialize};

// boot into a os (request)
// POST /api/op/boot
// cts
#[derive(Deserialize, Serialize)]
pub struct BootReq<'a> {
    pub mac_address: Cow<'a, [u8; 6]>,
    pub os: OSStatus,
}
// stc
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum BootRes {
    Success,
    Fail,
    NotFound,
}

// get a list of machine
// POST /api/get/machines
// cts: no payload
// stc
#[derive(Deserialize, Serialize)]
pub struct MachineList<'a> {
    pub machines: Vec<MachineInfoInner<'a>>,
}

// get detailed info of a machine
// POST /api/get/machine
// cts
#[derive(Deserialize, Serialize)]
pub struct MachineInfoReq<'a> {
    pub mac_address: Cow<'a, [u8; 6]>,
}
// stc
// return type is wrapped in option
pub type MachineInfo<'a> = Option<MachineInfoInner<'a>>;

// get a list of os
// POST /api/get/oss
// cts
#[derive(Deserialize, Serialize)]
pub struct OsListReq<'a> {
    pub mac_address: Cow<'a, [u8; 6]>,
}
// stc
#[derive(Deserialize, Serialize)]
pub struct OsList<'a> {
    pub oss: Vec<OsInfoInner<'a>>,
}

// // get detailed info of an os
// // POST /api/get/os
// // cts
// #[derive(Deserialize, Serialize)]
// pub struct OsInfoReq{
//     pub mac_address: [u8; 6],
//     pub os:ID
// }
// // stc
// pub type OsInfo<'a>=Option<OsInfoInner<'a>>;

// init new machine
// POST /api/op/new
// cts
#[derive(Deserialize, Serialize)]
pub struct NewMachineReq<'a> {
    pub display_name: Cow<'a, str>,
    pub mac_address: Cow<'a, [u8; 6]>,
}
// stc
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum NewMachineRes {
    Success,
    Fail,
    NotFound,
}

// login
// POST /login
// cts
#[derive(Deserialize, Serialize)]
pub struct LoginReq<'a> {
    pub password: Cow<'a, str>,
}
// stc
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum LoginRes {
    Success,
    Fail,
}

#[derive(Deserialize, Serialize)]
pub struct MachineInfoInner<'a> {
    pub display_name: Option<Cow<'a, str>>,
    pub mac_address: Cow<'a, [u8; 6]>,
    pub state: MachineState,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum MachineState {
    Down{
        kind:MustBe!("Down"),
    },
    Uninited{
        kind:MustBe!("Uninited"),
    },
    Up{
        kind:MustBe!("Up"),
        id:ID
    },
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum OSStatus {
    Down{
        kind:MustBe!("Down"),
    },
    Up{
        id:ID
    },
}

#[derive(Deserialize, Serialize)]
pub struct OsInfoInner<'a> {
    pub display_name: Cow<'a, str>,
    pub id: ID,
}
