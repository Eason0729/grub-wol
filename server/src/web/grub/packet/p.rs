use std::pin::Pin;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use std::{mem, time};

use super::event::EventHook;
use super::hashvec::HashVec;
use super::wol;
use async_std::future::timeout;
use async_std::net;
use async_std::prelude::FutureExt;
use futures_lite::Future;
use proto::prelude::packets as PacketType;
use proto::prelude::{self as protocal, host, server};
type MacAddress = [u8; 6];

type Conn = protocal::TcpConn<PacketType::server::Packet, PacketType::host::Packet>;

const TIMEOUT: u64 = 180;
const TIMEOUTSHORT: u64 = 5;
const TIMEOUTBUSY: u64 = 400;// ms

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

struct RawPacket {
    conn: Conn,
    uid: protocal::ID,
}
impl RawPacket {
    async fn from_conn_handshake(mut conn: Conn) -> Result<([u8; 6], Self), Error> {
        if let host::Packet::HandShake(handshake) =
            conn.read().await.map_err(|_| Error::UnknownProtocal)?
        {
            Self::from_handshake(conn, handshake).await
        } else {
            Err(Error::UnknownProtocal)
        }
    }

    async fn from_handshake(
        mut conn: Conn,
        handshake: host::HandShake,
    ) -> Result<([u8; 6], Self), Error> {
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

        Ok((mac_address, Self { conn, uid }))
    }
    fn fake_uid(&mut self,uid:protocal::ID){
        self.uid=uid;
    }
}

struct Info {
    mac_address: [u8; 6],
}
struct Packet<'a> {
    manager: &'a Packets,
    unused_receive: Mutex<HashVec<ReceivePacketType, host::Packet>>,
    info: Info,
    raw: RwLock<Option<RawPacket>>,
}

impl<'a> Packet<'a> {
    fn read_packet(&self) -> Result<RwLockReadGuard<Option<RawPacket>>,Error> {
        let raw=self.raw.read().unwrap();
        match &*raw {
            Some(_) => Ok(raw),
            None => Err(Error::ClientOffline),
        }
    }
    fn write_packet(&self) -> Result<RwLockWriteGuard<Option<RawPacket>>,Error> {
        let raw=self.raw.write().unwrap();
        match &*raw {
            Some(_) => Ok(raw),
            None => Err(Error::ClientOffline),
        }
    }
    pub fn get_handshake_uid(&self) -> Result<protocal::ID,Error> {
        Ok(self.read_packet()?.as_ref().unwrap().uid)
    }
    pub fn get_mac(&self) -> [u8; 6] {
        self.info.mac_address
    }
    async fn send(&self, package: server::Packet) -> Result<(),Error> {
        let mut packet=self.write_packet()?;
        let packet=packet.as_mut().unwrap();
        packet.conn.send(package).await?;
        Ok(())
    }
    async fn read(&self, packet_type: ReceivePacketType) -> Result<host::Packet, Error> {
        let mut packet=self.write_packet()?;
        let packet=packet.as_mut().unwrap();

        let mut unused = self.unused_receive.lock().unwrap();
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
    async fn read_timeout(
        &self,
        packet_type: ReceivePacketType,
    ) -> Result<host::Packet, Error> {
        timeout(
            time::Duration::from_secs(TIMEOUTSHORT),
            self.read(packet_type),
        )
        .await
        .map_err(|_| Error::Timeout)?
    }
    pub async fn issue_id(&self, id: protocal::ID) -> Result<(), Error> {
        self.send(server::Packet::InitId(id)).await?;
        self.read_timeout(ReceivePacketType::InitId).await?;
        self.fake_uid(id)?;
        Ok(())
    }
    fn fake_uid(&self, id: protocal::ID) -> Result<(), Error> {
        let mut packet=self.write_packet()?;
        let packet=packet.as_mut().unwrap();
        packet.uid = id;
        Ok(())
    }
    pub async fn wol_reconnect(&self, mac_address: &[u8; 6]) -> Result<(), Error> {
        timeout(
            time::Duration::from_secs(TIMEOUTSHORT),
            self.wait_reconnect(),
        )
        .await
        .map_err(|_| Error::Timeout)?
    }
    pub async fn wait_reconnect(&self) -> Result<(), Error> {
        let mut packet=self.write_packet()?;
        let packet=&mut *packet;
        *packet=None;

        let event_hook = &self.manager.event_hook;

        let new_packet = event_hook
            .timeout(self.info.mac_address, time::Duration::from_secs(TIMEOUT))
            .await
            .map_err(|_| Error::Timeout)?;

        let mut packet=self.write_packet()?;
        *packet=Some(new_packet);
        
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
    pub async fn boot_into(&self, grub_sec: protocal::Integer) -> Result<(), Error> {
        self.send(server::Packet::Reboot(grub_sec)).await?;
        self.read_timeout(ReceivePacketType::Reboot).await?;
        self.wol_reconnect(&self.get_mac()).await?;
        Ok(())
    }
    pub async fn os_query(&self) -> Result<host::OSInfo, Error> {
        self.send(server::Packet::OSQuery).await?;
        if let host::Packet::OSQuery(query) = self.read_timeout(ReceivePacketType::OSQuery).await? {
            Ok(query)
        } else {
            Err(Error::UnknownProtocal)
        }
    }
}

#[derive(Default)]
struct Packets {
    event_hook: EventHook<MacAddress, RawPacket>,
}

impl Packets {
    pub async fn connect<'a>(
        &'a self,
        stream: net::TcpStream,
    ) -> Result<Option<Packet<'a>>, Error> {
        let conn = Conn::from_tcp(stream);
        let (mac_address, raw_packet) = RawPacket::from_conn_handshake(conn).await?;
        match self
            .event_hook
            .signal(&mac_address, raw_packet)
        {
            Some( raw_packet) => Ok(Some(Packet {
                unused_receive: Mutex::new(HashVec::default()),
                manager: self,
                raw: RwLock::new(Some(raw_packet)),
                info: Info { mac_address },
            })),
            None => Ok(None),
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
    #[error("conn error")]
    Conn(#[from] protocal::Error),
}
