use async_std::{
    channel,
    future::timeout,
    io, net,
    sync::{Mutex, RwLock},
    task::spawn,
};
use futures_lite::Future;
use paste::paste;
use proto::prelude::{
    packets::host::Packet as HostP, packets::server::Packet as ServerP, Connection, ID,
};
use std::{
    collections::{HashMap, VecDeque},
    future::IntoFuture,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use super::{event::EventHook, hashvec::HashVec};

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
    T: io::WriteExt + Unpin + io::ReadExt,
{
    conn: Connection<ServerP, HostP, T, T>,
    buffer: HashVec<HostPTy, HostP>,
}

#[cfg(debug_assertions)]
impl<T> Drop for PacketIo<T>
where
    T: io::WriteExt + Unpin + io::ReadExt,
{
    fn drop(&mut self) {
        assert_eq!(0, self.buffer.len());
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
            conn: Connection::new(stream.clone(), stream),
            buffer: Default::default(),
        }
    }
    async fn read(self_: &Mutex<Self>, ty: HostPTy) -> Result<HostP, Error> {
        loop {
            // check if there is already one reader
            match self_.try_lock() {
                Some(mut self_) => {
                    // TODO: clear_posion if proto::transfer Error
                    let res = self_.conn.read().await?;
                    let res_ty = HostPTy::from_packet(&res);
                    if res_ty == ty {
                        return Ok(res);
                    } else {
                        self_.buffer.push(res_ty, res);
                    }
                }
                None => {
                    let mut self_ = self_.lock().await;
                    if let Some(x) = self_.buffer.pop(&ty) {
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

struct HandshakeInfo {
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

        let handshake = UnwrapEnum!(
            PacketIo::read(&conn, HostPTy::Handshake).await?,
            HostP::Handshake
        );
        let handshake = HandshakeInfo {
            uid: handshake.uid,
            mac_address: handshake.mac_address,
        };

        Ok(RawPacket { conn, handshake })
    }
}

pub struct Packet<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    raw: RwLock<Option<RawPacket<T>>>,
    event_hook: Arc<EventHook<[u8; 6], RawPacket<T>>>,
}

macro_rules! impl_packet {
    ($p:ident) => {
        paste! {
            pub async fn [<send_ $p:lower>](&self) -> Result<proto::prelude::host::$p, Error> {
                let conn = self.raw.read().await.as_ref().map(|x| x.conn.clone()).ok_or(Error::ClientOffline)?;
                let res = PacketIo::read(&conn, HostPTy::$p).await?;
                Ok(UnwrapEnum!(res,HostP::$p))
            }
        }
    };
}

macro_rules! impl_packet_signal {
    ($p:ident) => {
        paste! {
            pub async fn [<send_ $p:lower>](&self) -> Result<proto::prelude::host::$p, Error> {
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
    impl_packet! {GrubQuery}
    impl_packet! {Ping}
    impl_packet_signal! {Reboot}
    impl_packet_signal! {InitId}
    impl_packet_signal! {ShutDown}
    impl_packet! {OsQuery}
    pub async fn wait_reconnect(&self) -> Result<(), Error> {
        match self.raw.write().await.take() {
            Some(raw) => {
                let new_raw = self.event_hook.wait(raw.handshake.mac_address).await;
                *self.raw.write().await = Some(new_raw);
                Ok(())
            }
            None => Err(Error::ClientOffline),
        }
    }
    // async fn wait_reconnect_timeout(&self,dur: Duration) -> Result<(), Error> {
    //     background_timeout(self.wait_reconnect(), dur).await?
    // }
}

#[derive(Default)]
pub struct Packets<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    event_hook: Arc<EventHook<[u8; 6], RawPacket<T>>>,
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
