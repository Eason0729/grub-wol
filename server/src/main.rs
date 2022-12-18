pub mod control;
pub mod ulog;
pub mod web;

use proto::prelude as protocal;
use smol::block_on;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() {
    let control_server = control::Server::new();
    let port = protocal::SERVER_PORT;
    let control_socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);

    let ex = smol::LocalExecutor::new();

    block_on(control_server.listen(control_socket));
}
