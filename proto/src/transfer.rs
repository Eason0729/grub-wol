use async_std::io::{ReadExt, WriteExt};
use async_std::net;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::vec;
use thiserror;

// use crate::mock::MockTcpStream;

type PrefixType = crate::constant::PacketPrefix;
// TODO: use bincode option to limit max bytes
const MAXSIZE: PrefixType = 1048576;

lazy_static! {
    static ref PREFIX_SIZE: usize = bincode::serialize(&(0 as PrefixType)).unwrap().len();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bincode")]
    BincodeError(#[from] bincode::Error),
    #[error("Error from smol")]
    SmolIOError(#[from] async_std::io::Error),
    #[error("too large entity")]
    TooLargeEntity,
}

pub struct ReadConn<S,Ty> where S:ReadExt,Ty: for<'a> Deserialize<'a>{
    pub data_type:PhantomData<Ty>,
    pub stream:S
}

impl<S, Ty> ReadConn<S, Ty>
where S:ReadExt+Unpin,Ty: for<'a> Deserialize<'a>
{
    pub async fn read(&mut self) -> Result<Ty, Error> {
        let mut prefix_buffer = vec![0_u8; *PREFIX_SIZE];
        self.stream.read_exact(&mut prefix_buffer).await?;

        let size: PrefixType = bincode::deserialize(&prefix_buffer)?;

        if size > MAXSIZE {
            return Err(Error::TooLargeEntity);
        }

        let mut packet_buffer = vec![0_u8; size as usize];

        self.stream.read_exact(&mut packet_buffer).await?;

        let packet: Ty = bincode::deserialize(&packet_buffer)?;
        Ok(packet)
    }
}

pub struct WriteConn<S,Ty> where S:WriteExt+Unpin,Ty:Serialize{
    pub data_type:PhantomData<Ty>,
    pub stream:S
}

impl<S, Ty> WriteConn<S, Ty>
where S:WriteExt+Unpin,Ty:Serialize
{
    pub async fn flush(&mut self)-> Result<(), Error>{
        self.stream.flush().await?;
        Ok(())
    }
    pub async fn write(&mut self, packet: Ty) -> Result<(), Error> {
        let binary = bincode::serialize(&packet)?;
        let size = binary.len() as PrefixType;
        let mut size = bincode::serialize(&size)?;

        size.extend_from_slice(binary.as_slice());
        self.stream.write_all(&size).await?;

        Ok(())
    }
}