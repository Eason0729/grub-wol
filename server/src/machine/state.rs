// use std::marker::PhantomData;
// use std::{fs, io, net, path};

// use super::graph::Graph;
// use super::machine;

// use proto::prelude as protocal;
// use proto::prelude::{Answer, GrubData, GrubDescription, Packet, Request};
// use serde::{Deserialize, Serialize};
// use std::collections::*;

// type NodeId = usize;
// type MacAddress = [u8; 6];

// #[derive(Ord, PartialOrd, Clone, PartialEq, Eq)]
// enum Singal {
//     AcceptReboot,
//     GrubQueryFinished,
//     IsAlive,
//     IsSpecifiedOs,
// }

// #[derive(Serialize, Deserialize)]
// pub struct OperatingSystem {
//     uid: protocal::ID,
//     grub_sec: usize,
//     display_name: String,
// }

// #[derive(Debug)]
// pub enum Error {}

// // a stateful inited machine
// #[derive(Serialize, Deserialize)]
// struct Machine {
//     #[serde(skip)]
//     state: Option<State>,
// }

// impl Machine {
//     async fn new(conn: &mut protocal::TcpConn) -> Result<Self, Error> {
//         // first time hanskshake
//         if let protocal::Packet::Handshake(p) = conn.read().await.map_err(|e| todo!())? {};
//         todo!()
//     }
// }

// // a stateless machine state
// struct State {
//     alive_os: OperatingSystem,
//     conn: protocal::TcpConn,
//     session_id: usize,
//     // ignitor: event::EventHook<Singal>,
// }

// impl State {
//     fn new(conn: protocal::TcpConn) {}
//     async fn ping(&mut self) -> Result<bool, protocal::Error> {
//         // If host is down(or not connected to the server), conn is expected to get BrokenPipe
//         match self.conn.send(Packet::Request(Request::Alive)).await {
//             Ok(_) => Ok(true),
//             Err(err) => {
//                 if let protocal::Error::SmolIOError(_) = err {
//                     Ok(false)
//                 } else {
//                     Err(err)
//                 }
//             }
//         }
//     }
//     async fn check_os(&mut self) -> Result<protocal::ID, protocal::Error> {
//         todo!()
//     }
// }
