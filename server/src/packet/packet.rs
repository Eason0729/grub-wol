use std::mem;

use proto::prelude::packets as PacketType;
use proto::prelude::{self as protocal, host};
use smol::net;

use super::event::EventHook;

type MacAddress = [u8; 6];

type Conn = protocal::TcpConn<PacketType::server::Packet, PacketType::host::Packet>;

struct Packets {
    event_hook: EventHook<MacAddress, Conn>,
}

impl Packets {
    async fn connect<'a>(&'a self, stream: net::TcpStream) -> Result<Option<Packet<'a>>, Error> {
        let mut conn = Conn::from_tcp(stream);
        if let host::Packet::HandShake(handshake) =
            conn.read().await.map_err(|_| Error::UnknownProtocal)?
        {
            if handshake.version != protocal::APIVERSION {
                return Err(Error::IncompatibleVersion);
            }
            let mac_address = handshake.mac_address;
            let uid = handshake.uid;

            return if let Some(conn) = self.event_hook.signal(&mac_address, conn) {
                Ok(Some(Packet {
                    manager: self,
                    conn: Some(conn),
                    mac_address,
                }))
            } else {
                Ok(None)
            };
        } else {
            Err(Error::UnknownProtocal)
        }
    }
}

struct Packet<'a> {
    manager: &'a Packets,
    conn: Option<Conn>,
    mac_address: MacAddress,
}

impl<'a> Packet<'a> {
    fn disconnect(&mut self) -> Result<(), Error> {
        let mut conn = None;
        mem::swap(&mut conn, &mut self.conn);
        return if let Some(conn) = conn {
            conn.shutdown();
            Ok(())
        } else {
            Err(Error::ClientOffline)
        };
    }
    async fn wait_reconnect(&mut self) -> Result<(), Error> {
        self.disconnect()?;
        self.manager.event_hook.timeout(self.mac_address, todo!());
    }
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("data store disconnected")]
    ClientOffline,
    #[error("data store disconnected")]
    UnknownProtocal,
    #[error("data store disconnected")]
    SmolIOError(#[from] smol::io::Error),
    #[error("data store disconnected")]
    IncompatibleVersion,
}
