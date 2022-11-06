use protocol::Protocol;
use petgraph::Graph;
use crate::constant;

#[derive(Protocol, Debug, PartialEq)]
enum Event {
    Handshake(HandShake),
    Request(Request),
    Response(Request,Response),
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
    GrubQuery
}

#[derive(Protocol, Debug, PartialEq)]
enum Response {
    Success,
    // Graph(Graph<>)
}
