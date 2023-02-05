use async_std::{
    channel,
    future::timeout,
    io, net,
    sync::{Mutex, RwLock},
    task::{sleep, spawn},
};
use futures_lite::{future::or, Future};
use paste::paste;
use proto::prelude::{
    packets::host::Packet as HostP, packets::server::Packet as ServerP, Connection, ID,
};
use std::{pin::Pin, sync::Arc, time::Duration};

use super::{event::EventHook, hashvec::HashVec, wol::MagicPacket};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

macro_rules! UnwrapEnum {
    ($e:expr,$l:path) => {
        if let $l(a) = $e {
            a
        } else {
            panic!("mismatch enum when unwrapping {}", stringify!($pat)); // #2
        }
    };
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum HostPTy {
    GrubQuery,
    Ping,
    Handshake,
    Reboot,
    InitId,
    ShutDown,
    OsQuery,
}

impl HostPTy {
    fn from_packet(packet: &HostP) -> Self {
        match packet {
            HostP::Handshake(_) => HostPTy::Handshake,
            HostP::GrubQuery(_) => HostPTy::GrubQuery,
            HostP::Ping(_) => HostPTy::Ping,
            HostP::Reboot => HostPTy::Reboot,
            HostP::InitId => HostPTy::InitId,
            HostP::ShutDown => HostPTy::ShutDown,
            HostP::OsQuery(_) => HostPTy::OsQuery,
        }
    }
}

struct PacketIo<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    conn: Option<Connection<ServerP, HostP, T, T>>,
    read_buffer: HashVec<HostPTy, HostP>,
}

impl<T> Drop for PacketIo<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        assert_eq!(0, self.read_buffer.len());
        let conn = self.conn.take();
        spawn(async move {
            conn.unwrap().flush().await.ok();
        });
    }
}

impl<T> PacketIo<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    fn new(stream: T) -> Self
    where
        T: Clone,
    {
        Self {
            conn: Some(Connection::new(stream.clone(), stream)),
            read_buffer: Default::default(),
        }
    }
    async fn write(self_: &Mutex<Self>, package: ServerP) -> Result<(), Error> {
        let conn = &mut self_.lock().await.conn;
        conn.as_mut().unwrap().send(package).await?;
        Ok(())
    }
    async fn read(self_: &Mutex<Self>, ty: HostPTy) -> Result<HostP, Error> {
        loop {
            // check if there is already one reader
            match self_.try_lock() {
                Some(mut self_) => {
                    // TODO: clear_posion if proto::transfer Error
                    let res = self_.conn.as_mut().unwrap().read().await?;
                    let res_ty = HostPTy::from_packet(&res);
                    if res_ty == ty {
                        return Ok(res);
                    } else {
                        self_.read_buffer.push(res_ty, res);
                    }
                }
                None => {
                    let mut self_ = self_.lock().await;
                    if let Some(x) = self_.read_buffer.pop(&ty) {
                        return Ok(x);
                    }
                }
            }
            todo!()
        }
    }
    async fn read_arc(self_: Arc<Mutex<Self>>, ty: HostPTy) -> Result<HostP, Error> {
        Self::read(&*self_, ty).await
    }
    async fn read_timeout(
        self_: Arc<Mutex<Self>>,
        ty: HostPTy,
        dur: Duration,
    ) -> Result<HostP, Error> {
        // expect reader to keep reading (consume that type of packet) if timeout
        // task in background if timeout
        background_timeout(Self::read_arc(self_.clone(), ty), dur).await?
        // TODO: use io::timeout instead
    }
}

async fn background_timeout<T>(
    fut: impl Future<Output = T> + Send + 'static,
    dur: Duration,
) -> Result<T, Error>
where
    T: Send + 'static,
{
    let (s, r) = channel::bounded(1);
    spawn(async move {
        let res = fut.await;
        s.send(res).await.ok();
    });

    Ok((timeout(dur, r.recv()).await)
        .map_err(|_| Error::Timeout)?
        .unwrap())
}

pub struct HandshakeInfo {
    uid: ID,
    mac_address: [u8; 6],
}

struct RawPacket<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    conn: Arc<Mutex<PacketIo<T>>>,
    handshake: HandshakeInfo,
}

impl<T> RawPacket<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    async fn new(stream: T) -> Result<RawPacket<T>, Error>
    where
        T: Clone,
    {
        let conn = Arc::new(Mutex::new(PacketIo::new(stream)));

        // read handshake
        let handshake = UnwrapEnum!(
            PacketIo::read(&conn, HostPTy::Handshake).await?,
            HostP::Handshake
        );
        let handshake = HandshakeInfo {
            uid: handshake.uid,
            mac_address: handshake.mac_address,
        };

        // write handshake
        let handshake_server = ServerP::Handshake(proto::prelude::server::Handshake {
            ident: proto::prelude::PROTO_IDENT,
            version: proto::prelude::APIVERSION,
        });

        PacketIo::write(&conn, handshake_server).await?;
        log::trace!("Handshake finished");

        Ok(RawPacket { conn, handshake })
    }
}

pub struct Packet<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    raw: RwLock<Option<RawPacket<T>>>,
    event_hook: Arc<EventHook<[u8; 6], RawPacket<T>>>,
    mac_address: [u8; 6],
}

macro_rules! impl_write_packet {
    ($p:ident) => {
        paste! {
            pub async fn [<write_ $p:lower>](&self,package:proto::prelude::server::$p) -> Result<(), Error> {
                log::trace!("sending {}",stringify!($p));
                let conn = self.raw.read().await.as_ref().map(|x| x.conn.clone()).ok_or(Error::ClientOffline)?;
                PacketIo::write(&conn, ServerP::$p(package)).await?;
                Ok(())
            }
        }
    };
}

macro_rules! impl_write_packet_signal {
    ($p:ident) => {
        paste! {
            pub async fn [<write_ $p:lower>](&self) -> Result<(), Error> {
                log::trace!("sending {}",stringify!($p));
                let conn = self.raw.read().await.as_ref().map(|x| x.conn.clone()).ok_or(Error::ClientOffline)?;
                PacketIo::write(&conn, ServerP::$p).await?;
                Ok(())
            }
        }
    };
}

macro_rules! impl_read_packet {
    ($p:ident) => {
        paste! {
            pub async fn [<read_ $p:lower>](&self) -> Result<proto::prelude::host::$p, Error> {
                log::trace!("received {}",stringify!($p));
                let conn = self.raw.read().await.as_ref().map(|x| x.conn.clone()).ok_or(Error::ClientOffline)?;
                let res = PacketIo::read(&conn, HostPTy::$p).await?;
                Ok(UnwrapEnum!(res,HostP::$p))
            }
        }
    };
}

macro_rules! impl_read_packet_signal {
    ($p:ident) => {
        paste! {
            pub async fn [<read_ $p:lower>](&self) -> Result<proto::prelude::host::$p, Error> {
                log::trace!("received {}",stringify!($p));
                let conn = self.raw.read().await.as_ref().map(|x| x.conn.clone()).ok_or(Error::ClientOffline)?;
                PacketIo::read(&conn, HostPTy::$p).await?;
                Ok(())
            }
        }
    };
}

impl<T> Packet<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    impl_write_packet! {Reboot}
    impl_write_packet! {InitId}
    impl_write_packet_signal! {ShutDown}
    impl_write_packet_signal! {GrubQuery}
    impl_write_packet_signal! {Ping}
    impl_write_packet_signal! {OsQuery}

    impl_read_packet! {GrubQuery}
    impl_read_packet! {Ping}
    impl_read_packet_signal! {Reboot}
    impl_read_packet_signal! {InitId}
    impl_read_packet_signal! {ShutDown}
    impl_read_packet! {OsQuery}

    pub async fn wait_reconnect(&self) -> Result<(), Error> {
        match self.raw.write().await.take() {
            Some(raw) => {
                let new_raw = self.event_hook.wait(raw.handshake.mac_address).await;
                *self.raw.write().await = Some(new_raw);
                log::debug!("received distributed RawPacket");
                Ok(())
            }
            None => Err(Error::ClientOffline),
        }
    }
    pub fn get_mac_address(&self) -> &[u8; 6] {
        &self.mac_address
    }
    pub async fn get_uid(&self) -> Result<ID, Error> {
        let raw = self.raw.read().await;
        let raw = raw.as_ref().ok_or(Error::ClientOffline)?;
        Ok(raw.handshake.uid)
    }
    pub fn set_uid(&mut self, uid: ID) -> Result<(), Error> {
        self.raw
            .get_mut()
            .as_mut()
            .ok_or(Error::ClientOffline)?
            .handshake
            .uid = uid;
        Ok(())
    }
    pub async fn wol_reconnect(&self) -> Result<(), Error> {
        let magic_packet = MagicPacket::new(self.get_mac_address());
        or(
            async {
                loop {
                    magic_packet.send().await;
                    sleep(Duration::from_millis(500)).await;
                }
            },
            self.wait_reconnect(),
        )
        .await?;
        Ok(())
    }
}

pub struct Packets<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    event_hook: Arc<EventHook<[u8; 6], RawPacket<T>>>,
}

impl<T> Default for Packets<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    fn default() -> Self {
        Self {
            event_hook: Default::default(),
        }
    }
}

impl<T> Packets<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    pub async fn connect(&self, stream: T) -> Result<Option<Packet<T>>, Error>
    where
        T: Clone,
    {
        let raw = RawPacket::new(stream).await?;
        let mac_address = raw.handshake.mac_address;
        match self.event_hook.signal(&mac_address, raw) {
            Some(raw) => Ok(Some(Packet {
                raw: RwLock::new(Some(raw)),
                event_hook: self.event_hook.clone(),
                mac_address,
            })),
            None => Ok(None),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("client offline")]
    ClientOffline,
    #[error("timeout")]
    Timeout,
    #[error("conn error")]
    Conn(#[from] proto::prelude::Error),
}

pub type TcpPacket = Packet<net::TcpStream>;
pub type TcpPackets = Packets<net::TcpStream>;
