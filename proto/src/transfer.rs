use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net;
use std::vec;
use std::{error, marker::PhantomData};
use nanoserde::{DeBin,SerBin,DeBinErr};
use thiserror;

type PrefixType = crate::constant::PacketPrefix;

lazy_static! {
    static ref PREFIX_SIZE: usize = PrefixType::serialize_bin(&0).len();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error from smol")] 
    SmolIOError(#[from] smol::io::Error),
    #[error("nanoserde")]
    Nanoserde(#[from] nanoserde::DeBinErr),
}

pub struct Connection<UP, DP, U, D>
where
    UP: SerBin,
    DP:DeBin,
    U: AsyncWriteExt + Unpin,
    D: AsyncReadExt + Unpin,
{
    upstream: U,
    downstream: D,
    upstream_packet: PhantomData<UP>,
    downstream_packet: PhantomData<DP>,
}

impl<UP, DP, U, D> Connection<UP, DP, U, D>
where
    UP: SerBin,
    DP: DeBin,
    U: AsyncWriteExt + Unpin,
    D: AsyncReadExt + Unpin,
{
    pub async fn send(&mut self, packet: UP) -> Result<(), Error> {
        let binary= UP::serialize_bin(&packet);
        let size = binary.len() as PrefixType;

        let mut size = PrefixType::serialize_bin(&size);
        
        size.extend_from_slice(binary.as_slice());
        self.upstream.write_all(&size).await?;

        Ok(())
    }
    pub async fn read(&mut self) -> Result<DP, Error> {
        let mut prefix_buffer = vec![0_u8; *PREFIX_SIZE];
        self.downstream.read_exact(&mut prefix_buffer).await?;

        let size: PrefixType = PrefixType::deserialize_bin(&prefix_buffer)?;

        let mut packet_buffer = vec![0_u8; size as usize];

        self.downstream.read_exact(&mut packet_buffer).await?;

        let packet: DP = DP::deserialize_bin(&packet_buffer)?;
        Ok(packet)
    }
    pub fn shutdown(self) {
        drop(self.upstream);
        drop(self.downstream);
    }
}

impl<UP, DP> Connection<UP, DP, net::TcpStream, net::TcpStream>
where
    UP: SerBin,
    DP: DeBin,
{
    pub fn from_tcp(stream: net::TcpStream) -> Self {
        Self {
            upstream: stream.clone(),
            downstream: stream,
            upstream_packet: Default::default(),
            downstream_packet: Default::default(),
        }
    }
}

pub type TcpConn<U, D> = Connection<U, D, net::TcpStream, net::TcpStream>;

#[cfg(test)]
mod test{
    fn basic(){
        
    }
}