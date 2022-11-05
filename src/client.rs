use crate::status::*;
use std::{collections::LinkedList, net};

struct Client {
    id: ID,
    server_socket_address: net::SocketAddrV4,
}

impl Client {
    fn new(id: usize, port: usize) -> Self {
        // try to find a server with matching id though dns-sd
        todo!()
    }
}
