use std::sync::Arc;
use std::time;
use std::time::Duration;

use super::event::EventHook;
use super::hashvec::HashVec;
use super::wol;
use async_std::{
    future::timeout,
    net,
    sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::sleep,
};
use futures_lite::future::race;
use proto::prelude::packets as PacketType;
use proto::prelude::{self as protocal, host, server};
type MacAddress = [u8; 6];

type Conn = protocal::TcpConn<PacketType::server::Packet, PacketType::host::Packet>;

const TIMEOUTLONG: u64 = 3600;
const TIMEOUT: u64 = 180;
const TIMEOUTSHORT: u64 = 50;

#[derive(Hash, Eq, PartialEq, Clone)]
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
            host::Packet::Handshake(_) => ReceivePacketType::Invaild,
            host::Packet::GrubQuery(_) => ReceivePacketType::GrubQuery,
            host::Packet::Ping(_) => ReceivePacketType::Ping,
            host::Packet::Reboot => ReceivePacketType::Reboot,
            host::Packet::InitId => ReceivePacketType::InitId,
            host::Packet::ShutDown => ReceivePacketType::ShutDown,
            host::Packet::OSQuery(_) => ReceivePacketType::OSQuery,
        }
    }
}

struct RawPacket {
    conn: Conn,
    uid: protocal::ID,
}

#[cfg(debug_assertions)]
impl Drop for RawPacket {
    fn drop(&mut self) {
        log::info!("RawPacket drop here");
    }
}
impl RawPacket {
    async fn from_conn_handshake(mut conn: Conn) -> Result<([u8; 6], Self), Error> {
        if let host::Packet::Handshake(handshake) =
            conn.read().await.map_err(|_| Error::UnknownProtocal)?
        {
            Self::from_handshake(conn, handshake).await
        } else {
            Err(Error::UnknownProtocal)
        }
    }

    async fn from_handshake(
        mut conn: Conn,
        handshake: host::Handshake,
    ) -> Result<([u8; 6], Self), Error> {
        if handshake.ident != protocal::PROTO_IDENT {
            return Err(Error::UnknownProtocal);
        }
        if handshake.version != protocal::APIVERSION {
            return Err(Error::IncompatibleVersion);
        }
        let mac_address = handshake.mac_address;
        let uid = handshake.uid;

        let server_handshake = server::Handshake {
            ident: protocal::PROTO_IDENT,
            version: protocal::APIVERSION,
        };

        conn.send(server::Packet::Handshake(server_handshake))
            .await
            .map_err(|_| Error::UnknownProtocal)?;

        Ok((mac_address, Self { conn, uid }))
    }
    fn fake_uid(&mut self, uid: protocal::ID) {
        self.uid = uid;
    }
}

struct Info {
    mac_address: [u8; 6],
}
pub struct Packet {
    event_hook: Arc<EventHook<MacAddress, RawPacket>>,
    unused_receive: Mutex<HashVec<ReceivePacketType, host::Packet>>,
    info: Info,
    raw: RwLock<Option<RawPacket>>,
}

impl Packet {
    async fn read_packet(&self) -> Result<RwLockReadGuard<Option<RawPacket>>, Error> {
        let raw = self.raw.read().await;
        match &*raw {
            Some(_) => Ok(raw),
            None => Err(Error::ClientOffline),
        }
    }
    async fn write_packet(&self) -> Result<RwLockWriteGuard<Option<RawPacket>>, Error> {
        let raw = self.raw.write().await;
        match &*raw {
            Some(_) => Ok(raw),
            None => Err(Error::ClientOffline),
        }
    }
    pub async fn get_handshake_uid(&self) -> Result<protocal::ID, Error> {
        Ok(self.read_packet().await?.as_ref().unwrap().uid)
    }
    pub fn get_mac(&self) -> [u8; 6] {
        self.info.mac_address
    }
    async fn send(&self, package: server::Packet) -> Result<(), Error> {
        let mut packet = self.write_packet().await?;
        let packet = packet.as_mut().unwrap();
        packet.conn.send(package).await?;
        Ok(())
    }
    async fn read(&self, packet_type: ReceivePacketType) -> Result<host::Packet, Error> {
        let mut packet = self.write_packet().await?;
        let packet = packet.as_mut().unwrap();

        let mut unused = self.unused_receive.lock().await;
        if let Some(packet) = unused.pop(&packet_type) {
            return Ok(packet);
        } else {
            loop {
                let package = packet
                    .conn
                    .read()
                    .await
                    .map_err(|_| Error::ClientDisconnect)?;
                let receive_type = ReceivePacketType::from_packet(&package);

                if receive_type == packet_type {
                    return Ok(package);
                } else {
                    unused.push(receive_type, package);
                }
            }
        }
    }
    async fn read_timeout(&self, packet_type: ReceivePacketType) -> Result<host::Packet, Error> {
        timeout(
            time::Duration::from_secs(TIMEOUTSHORT),
            self.read(packet_type),
        )
        .await
        .map_err(|_| {
            log::error!("unexpected timeout");
            Error::Timeout
        })?
    }
    pub async fn issue_id(&self, id: protocal::ID) -> Result<(), Error> {
        self.send(server::Packet::InitId(id)).await?;
        self.read_timeout(ReceivePacketType::InitId).await?;
        self.fake_uid(id).await?;
        Ok(())
    }
    async fn fake_uid(&self, id: protocal::ID) -> Result<(), Error> {
        let mut packet = self.write_packet().await?;
        let packet = packet.as_mut().unwrap();
        packet.uid = id;
        Ok(())
    }
    pub async fn wol_reconnect(&self) -> Result<(), Error> {
        race(
            async {
                loop {
                    wol::MagicPacket::new(&self.info.mac_address).send().await;
                    sleep(Duration::from_secs(1)).await;
                }
            },
            self.wait_reconnect(),
        )
        .await
    }
    pub async fn wait_reconnect(&self) -> Result<(), Error> {
        let event_hook = &self.event_hook;

        let new_packet = event_hook
            .timeout(self.info.mac_address, time::Duration::from_secs(TIMEOUT))
            .await
            .map_err(|_| Error::Timeout)?;

        let mut packet = self.write_packet().await?;
        *packet = Some(new_packet);
        // RawPacket drop here

        Ok(())
    }
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.send(server::Packet::ShutDown).await?;
        self.read_timeout(ReceivePacketType::ShutDown).await?;
        Ok(())
    }
    pub async fn grub_query(&self) -> Result<Vec<host::GrubInfo>, Error> {
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
    pub async fn boot_into(&mut self, grub_sec: protocal::GrubId) -> Result<(), Error> {
        self.send(server::Packet::Reboot(grub_sec)).await?;
        self.read_timeout(ReceivePacketType::Reboot).await?;
        self.wol_reconnect().await?;
        Ok(())
    }
    pub async fn os_query(&self) -> Result<host::OSQuery, Error> {
        self.send(server::Packet::OSQuery).await?;
        log::debug!("after sending query");
        if let host::Packet::OSQuery(query) = self.read_timeout(ReceivePacketType::OSQuery).await? {
            log::debug!("after reading query");
            Ok(query)
        } else {
            Err(Error::UnknownProtocal)
        }
    }
}

#[derive(Default)]
pub struct Packets {
    event_hook: Arc<EventHook<MacAddress, RawPacket>>,
}

impl Packets {
    pub async fn connect(&self, stream: net::TcpStream) -> Result<Option<Packet>, Error> {
        let conn = Conn::from_tcp(stream);
        let (mac_address, raw_packet) = RawPacket::from_conn_handshake(conn).await?;
        log::trace!(
            "Client with mac address {:x?} has finished hanshake",
            &mac_address
        );
        match self.event_hook.signal(&mac_address, raw_packet) {
            Some(raw_packet) => Ok(Some(Packet {
                unused_receive: Mutex::new(HashVec::default()),
                event_hook: self.event_hook.clone(),
                raw: RwLock::new(Some(raw_packet)),
                info: Info { mac_address },
            })),
            None => Ok(None),
        }
    }
    pub fn unconnected(&self, mac_address: [u8; 6]) -> Result<Packet, Error> {
        Ok(Packet {
            unused_receive: Mutex::new(HashVec::default()),
            event_hook: self.event_hook.clone(),
            raw: RwLock::new(None),
            info: Info { mac_address },
        })
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
    #[error("conn error")]
    Conn(#[from] protocal::Error),
}
