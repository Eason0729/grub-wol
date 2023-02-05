use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use paste::paste;
use async_std::{channel, future::timeout, io, net, sync::Mutex, task::spawn};
use proto::prelude::{
    packets::host::Packet as HostP, packets::server::Packet as ServerP, Connection, ID,
};

use super::{event::EventHook, hashvec::HashVec};

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
    OSQuery,
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
            HostP::OSQuery(_) => HostPTy::OSQuery,
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
        let (s, r) = channel::unbounded();
        spawn(async move {
            let res = Self::read_arc(self_.clone(), ty).await;
            s.send(res).await.ok();
        });

        (timeout(dur, r.recv()).await)
            .map_err(|_| Error::Timeout)?
            .unwrap()
        // TODO: use io::timeout instead
    }
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

struct Packet<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    raw: Option<RawPacket<T>>,
    event_hook: Arc<EventHook<[u8; 6], RawPacket<T>>>,
}

macro_rules! impl_packet {
    ($p:ident) => {
        paste!{
            async fn [<send_ $p:lower>](&self) -> Result<proto::prelude::host::$p, Error> {
                let conn = self
                    .raw
                    .as_ref()
                    .map(|x| x.conn.clone())
                    .ok_or(Error::ClientOffline)?;
                let res = PacketIo::read(&conn, HostPTy::$p).await?;
        
                Ok(UnwrapEnum!(res,HostP::$p))
            }
        }
    }
}

impl<T> Packet<T>
where
    T: io::WriteExt + Unpin + io::ReadExt + Send + 'static,
{
    impl_packet!{OSQuery}
    impl_packet!{GrubQuery}
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
