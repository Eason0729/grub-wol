use super::*;
use crate::constant;
use petgraph::Graph;
use protocol::Protocol;

#[derive(Protocol, Debug, PartialEq)]
enum Event {
    Handshake(HandShake),
    Request(Request),
    Response(Request, Response),
}

#[derive(Protocol, Debug, PartialEq)]
struct HandShake {
    mac_address: [u8; 6],
    client_id: constant::ID,
}

#[derive(Protocol, Debug, PartialEq)]
enum Request {
    ShutDown,
    GrubReboot,
    GrubQuery,
}

#[derive(Protocol, Debug, PartialEq)]
enum Response {
    Success,
    GrubData(GrubData),
}

#[derive(Protocol, Debug, PartialEq)]
struct GrubData {
    list: Vec<OperatingSystem>,
}

#[derive(Protocol, Debug, PartialEq)]
pub struct OperatingSystem {
    grub_sec: constant::Integer,
    name: String,
}
