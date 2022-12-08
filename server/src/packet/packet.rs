// use std::mem;

// use proto::prelude as protocal;

// use super::event::EventHook;

// type MacAddress = [u8; 6];

// struct Packets {
//     event_hook: EventHook<MacAddress, protocal::Conn>,
// }

// impl Packets {
//     async fn connect<'a>(&self, mut conn: protocal::Conn) -> Result<Packet<'a>, Error> {
//         if let protocal::Packet::Handshake(handshake) =
//             conn.read().await.map_err(|err| match err {
//                 protocal::Error::PacketSerializeError(_)
//                 | protocal::Error::PrefixSerializeError(_) => Error::UnknownProtocal,
//                 protocal::Error::SmolIOError(e) => Error::SmolIOError(e),
//             })?
//         {
//             if handshake.version != protocal::APIVERSION {
//                 return Err(Error::IncompatibleVersion);
//             }
//             let mac_address = handshake.mac_address;
//             let uid = handshake.uid;
//             self.event_hook.signal(&mac_address);
//         };
//         todo!()
//     }
// }

// struct Packet<'a> {
//     manager: &'a Packets,
//     conn: Option<protocal::Conn>,
// }

// impl<'a> Packet<'a> {
//     fn disconnect(&mut self) -> Result<(), Error> {
//         let mut conn = None;
//         mem::swap(&mut conn, &mut self.conn);
//         return if let Some(conn) = conn {
//             conn.shutdown();
//             Ok(())
//         } else {
//             Err(Error::ClientOffline)
//         };
//     }
//     async fn wait_reconnect(&mut self) {}
// }

// enum Error {
//     ClientOffline,
//     UnknownProtocal,
//     SmolIOError(smol::io::Error),
//     IncompatibleVersion,
// }
