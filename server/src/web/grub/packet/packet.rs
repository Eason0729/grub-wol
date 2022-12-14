use std::{mem, time};

use proto::prelude::packets as PacketType;
use proto::prelude::{self as protocal, host, server};
use smol::future::{or, Or};
use smol::{net, Timer};

use super::event::EventHook;
use super::hashvec::HashVec;
use super::wol;

type MacAddress = [u8; 6];

type Conn = protocal::TcpConn<PacketType::server::Packet, PacketType::host::Packet>;

const TIMEOUT: u64 = 180;
const TIMEOUTSHORT: u64 = 5;

macro_rules! ok_or_ref {
    ($i:expr,$e:expr) => {
        match &mut $i {
            Some(x) => Ok(x),
            None => Err($e),
        }?
    };
}

/// A packet manager
#[derive(Default)]
pub struct Packets {
    event_hook: EventHook<MacAddress, RawPacket>,
}

impl Packets {
    pub async fn connect<'a>(
        &'a self,
        stream: net::TcpStream,
    ) -> Result<Option<Packet<'a>>, Error> {
        let conn = Conn::from_tcp(stream);
        let raw_packet = RawPacket::from_conn_handshake(conn).await?;
        match self
            .event_hook
            .signal(&raw_packet.mac_address.clone(), raw_packet)
        {
            Some(raw_packet) => Ok(Some(Packet {
                unused_receive: HashVec::default(),
                manager: self,
                raw: Some(raw_packet),
            })),
            None => Ok(None),
        }
    }
}

/// raw packet(lack of EventHook support), only do handshake
struct RawPacket {
    conn: Conn,
    mac_address: MacAddress,
    uid: protocal::ID,
}

impl RawPacket {
    async fn from_conn_handshake(mut conn: Conn) -> Result<Self, Error> {
        if let host::Packet::HandShake(handshake) =
            conn.read().await.map_err(|_| Error::UnknownProtocal)?
        {
            Self::from_handshake(conn, handshake).await
        } else {
            Err(Error::UnknownProtocal)
        }
    }

    async fn from_handshake(mut conn: Conn, handshake: host::HandShake) -> Result<Self, Error> {
        if handshake.ident != protocal::PROTO_IDENT {
            return Err(Error::UnknownProtocal);
        }
        if handshake.version != protocal::APIVERSION {
            return Err(Error::IncompatibleVersion);
        }
        let mac_address = handshake.mac_address;
        let uid = handshake.uid;

        let server_handshake = server::HandShake {
            ident: protocal::PROTO_IDENT,
            version: protocal::APIVERSION,
        };

        conn.send(server::Packet::HandShake(server_handshake))
            .await
            .map_err(|_| Error::UnknownProtocal)?;

        Ok(Self {
            conn,
            mac_address,
            uid,
        })
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum ReceivePacketType {
    GrubQuery,
    Ping,
    Invaild,
    Reboot,
    InitId,
    ShutDown,
    OSQuery,
}

impl ReceivePacketType {
    fn from_packet(packet: &host::Packet) -> Self {
        match packet {
            host::Packet::HandShake(_) => ReceivePacketType::Invaild,
            host::Packet::GrubQuery(_) => ReceivePacketType::GrubQuery,
            host::Packet::Ping(_) => ReceivePacketType::Ping,
            host::Packet::Reboot => ReceivePacketType::Reboot,
            host::Packet::InitId => ReceivePacketType::InitId,
            host::Packet::ShutDown => ReceivePacketType::ShutDown,
            host::Packet::OSQuery(_) => ReceivePacketType::OSQuery,
        }
    }
}

/// managed packet
pub struct Packet<'a> {
    manager: &'a Packets,
    unused_receive: HashVec<ReceivePacketType, host::Packet>,
    raw: Option<RawPacket>,
}

impl<'a> Packet<'a> {
    pub fn get_handshake_uid(&mut self) -> Result<protocal::ID, Error> {
        let raw = ok_or_ref!(self.raw, Error::ClientOffline);
        Ok(raw.uid)
    }
    pub fn get_mac(&mut self) -> Result<MacAddress, Error> {
        let raw = ok_or_ref!(self.raw, Error::ClientOffline);
        Ok(raw.mac_address)
    }
    fn fake_handshake_uid(&mut self, id: protocal::ID) -> Result<(), Error> {
        let raw = ok_or_ref!(self.raw, Error::ClientOffline);
        raw.uid = id;
        Ok(())
    }
    pub async fn wol_reconnect(&mut self, mac_address: &[u8; 6]) -> Result<(), Error> {
        or(self.wait_reconnect(), async {
            let magic_packet = wol::MagicPacket::new(mac_address);
            loop {
                Timer::after(time::Duration::from_secs(1)).await;
                magic_packet.send();
            }
        })
        .await
    }
    pub async fn wait_reconnect(&mut self) -> Result<(), Error> {
        let mut raw = None;
        mem::swap(&mut raw, &mut self.raw);
        let raw = raw.unwrap();

        let event_hook = &self.manager.event_hook;

        let raw_packet = event_hook
            .timeout(raw.mac_address, time::Duration::from_secs(TIMEOUT))
            .await
            .map_err(|_| Error::Timeout)?;

        self.raw = Some(raw_packet);
        Ok(())
    }
    async fn send(&mut self, packet: server::Packet) -> Result<(), Error> {
        ok_or_ref!(self.raw, Error::ClientOffline)
            .conn
            .send(packet)
            .await
            .map_err(|_| Error::ClientDisconnect)?;
        Ok(())
    }
    async fn read_timeout(
        &mut self,
        packet_type: ReceivePacketType,
    ) -> Result<host::Packet, Error> {
        let mut package = None;
        or(
            async {
                package = Some(self.read(packet_type).await);
            },
            async {
                Timer::after(time::Duration::from_secs(TIMEOUTSHORT));
            },
        )
        .await;
        match package {
            Some(x) => Ok(x?),
            None => Err(Error::Timeout),
        }
    }
    async fn read(&mut self, packet_type: ReceivePacketType) -> Result<host::Packet, Error> {
        if let Some(packet) = self.unused_receive.pop(&packet_type) {
            Ok(packet)
        } else {
            loop {
                let packet = ok_or_ref!(self.raw, Error::ClientOffline)
                    .conn
                    .read()
                    .await
                    .map_err(|_| Error::ClientDisconnect)?;
                let receive_type = ReceivePacketType::from_packet(&packet);

                if receive_type == packet_type {
                    return Ok(packet);
                } else {
                    self.unused_receive.push(receive_type, packet);
                }
            }
        }
    }
    pub async fn shutdown(&mut self) -> Result<(), Error> {
        self.send(server::Packet::ShutDown).await?;
        self.read_timeout(ReceivePacketType::ShutDown).await?;
        Ok(())
    }
    pub async fn grub_query(&mut self) -> Result<Vec<host::GrubInfo>, Error> {
        // TODO: grub-probe is expect to keep running for longer time, add extra waiting time
        self.send(server::Packet::GrubQuery).await?;
        if let host::Packet::GrubQuery(query) =
            self.read_timeout(ReceivePacketType::GrubQuery).await?
        {
            Ok(query)
        } else {
            Err(Error::UnknownProtocal)
        }
    }
    pub async fn boot_into(&mut self, grub_sec: protocal::Integer) -> Result<(), Error> {
        self.send(server::Packet::Reboot(grub_sec)).await?;
        self.read_timeout(ReceivePacketType::Reboot).await?;
        Ok(())
    }
    pub async fn issue_id(&mut self, id: protocal::ID) -> Result<(), Error> {
        self.send(server::Packet::InitId(id)).await?;
        self.read_timeout(ReceivePacketType::InitId).await?;
        self.fake_handshake_uid(id)?;
        Ok(())
    }
    pub async fn ping(&mut self) -> Result<(), Error> {
        self.send(server::Packet::Ping).await?;
        if let host::Packet::Ping(id) = self.read_timeout(ReceivePacketType::Ping).await? {
            if self.raw.as_ref().ok_or(Error::ClientOffline)?.uid != id {
                self.raw = None;
                Err(Error::ClientOffline)
            } else {
                Ok(())
            }
        } else {
            Err(Error::UnknownProtocal)
        }
    }
    pub async fn os_query(&mut self) -> Result<host::OSInfo, Error> {
        self.send(server::Packet::OSQuery).await?;
        if let host::Packet::OSQuery(query) = self.read_timeout(ReceivePacketType::OSQuery).await? {
            Ok(query)
        } else {
            Err(Error::UnknownProtocal)
        }
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
