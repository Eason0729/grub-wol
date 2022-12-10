use std::{mem, time};

use proto::prelude::packets as PacketType;
use proto::prelude::{self as protocal, host, server};
use smol::net;

use super::event::EventHook;

type MacAddress = [u8; 6];

type Conn = protocal::TcpConn<PacketType::server::Packet, PacketType::host::Packet>;

const TIMEOUT: u64 = 180;

macro_rules! ok_or_ref {
    ($i:expr,$e:expr) => {
        match &mut $i {
            Some(x) => Ok(x),
            None => Err($e),
        }?
    };
}

#[derive(Default)]
pub struct Packets {
    event_hook: EventHook<MacAddress, Conn>,
}

impl Packets {
    pub async fn connect<'a>(
        &'a self,
        stream: net::TcpStream,
    ) -> Result<Option<Packet<'a>>, Error> {
        let mut conn = Conn::from_tcp(stream);
        if let host::Packet::HandShake(handshake) =
            conn.read().await.map_err(|_| Error::UnknownProtocal)?
        {
            if handshake.version != protocal::APIVERSION {
                return Err(Error::IncompatibleVersion);
            }
            let mac_address = handshake.mac_address;
            let uid = handshake.uid;

            let server_handshake = server::HandShake {
                version: protocal::APIVERSION,
            };
            conn.send(server::Packet::HandShake(server_handshake))
                .await
                .map_err(|_| Error::UnknownProtocal)?;

            if let Some(conn) = self.event_hook.signal(&mac_address, conn) {
                Ok(Some(Packet {
                    manager: self,
                    conn: Some(conn),
                    mac_address,
                }))
            } else {
                Ok(None)
            }
        } else {
            Err(Error::UnknownProtocal)
        }
    }
}

pub struct Packet<'a> {
    manager: &'a Packets,
    conn: Option<Conn>,
    mac_address: MacAddress,
}

impl<'a> Packet<'a> {
    pub fn disconnect(&mut self) -> Result<(), Error> {
        let mut conn = None;
        mem::swap(&mut conn, &mut self.conn);
        return if let Some(conn) = conn {
            conn.shutdown();
            Ok(())
        } else {
            Err(Error::ClientOffline)
        };
    }
    pub async fn wait_reconnect(&mut self) -> Result<(), Error> {
        self.disconnect()?;
        let conn = self
            .manager
            .event_hook
            .timeout(self.mac_address, time::Duration::from_secs(TIMEOUT))
            .await
            .map_err(|_| Error::Timeout)?;
        self.conn = Some(conn);
        Ok(())
    }
    async fn send(&mut self, packet: server::Packet) -> Result<(), Error> {
        ok_or_ref!(self.conn, Error::ClientOffline)
            .send(packet)
            .await
            .map_err(|_| Error::ClientDisconnect)?;
        Ok(())
    }
    pub async fn shutdown(&mut self) -> Result<(), Error> {
        self.send(server::Packet::ShutDown).await
    }
    pub async fn grub_query(&mut self) -> Result<(), Error> {
        self.send(server::Packet::GrubQuery).await
    }
    pub async fn boot_into(&mut self, grub_sec: protocal::ID) -> Result<(), Error> {
        self.send(server::Packet::Reboot(grub_sec)).await
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Client may have never connected")]
    ClientOffline,
    #[error("Client disconnected during operation")]
    ClientDisconnect,
    #[error("Unknown device hit the socket")]
    UnknownProtocal,
    #[error("client api version must match")]
    IncompatibleVersion,
    #[error("timeout")]
    Timeout,
}
