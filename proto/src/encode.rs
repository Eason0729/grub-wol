use smol::net;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use smol::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub enum Error {
    PacketSerializeError(bincode::Error),
    PrefixSerializeError(bincode::Error),
    SmolIOError(smol::io::Error),
}

type PrefixType = crate::constant::PacketPrefix;

pub type TcpConn<P> = Connection<net::TcpStream, net::TcpStream, P>;

lazy_static! {
    static ref PREFIX_SIZE: usize = bincode::serialize(&(0 as PrefixType)).unwrap().len();
}

pub struct Connection<R, W, P>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
    P: Serialize + for<'a> Deserialize<'a>,
{
    reader: R,
    writer: W,
    data: PhantomData<P>,
}

impl<P> Connection<net::TcpStream, net::TcpStream, P>
where
    P: Serialize + for<'a> Deserialize<'a>,
{
    pub fn from_tcp(stream: &net::TcpStream) -> Connection<net::TcpStream, net::TcpStream, P> {
        Self {
            reader: stream.clone(),
            writer: stream.clone(),
            data: PhantomData,
        }
    }
}

impl<R, W, P> Connection<R, W, P>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
    P: Serialize + for<'a> Deserialize<'a>,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            data: PhantomData,
        }
    }
    pub async fn send(&mut self, packet: P) -> Result<(), Error> {
        let binary = bincode::serialize(&packet).map_err(|e| Error::PacketSerializeError(e))?;
        let size = binary.len() as PrefixType;
        let size = bincode::serialize(&size).unwrap();

        self.writer
            .write_all(&size)
            .await
            .map_err(|e| Error::SmolIOError(e))?;
        self.writer
            .write_all(&binary)
            .await
            .map_err(|e| Error::SmolIOError(e))?;
        Ok(())
    }
    pub async fn read(&mut self) -> Result<P, Error> {
        let mut prefix_buffer = vec![0_u8; *PREFIX_SIZE];
        self.reader
            .read_exact(&mut prefix_buffer)
            .await
            .map_err(|e| Error::SmolIOError(e))?;

        let size: PrefixType =
            bincode::deserialize(&prefix_buffer).map_err(|e| Error::PrefixSerializeError(e))?;

        let mut packet_buffer = vec![0_u8; size as usize];

        self.reader
            .read_exact(&mut packet_buffer)
            .await
            .map_err(|e| Error::SmolIOError(e))?;

        let packet: P =
            bincode::deserialize(&packet_buffer).map_err(|e| Error::PacketSerializeError(e))?;

        Ok(packet)
    }
    pub fn shutdown(self){
        drop(self);
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;
    use smol::net;
    use smol::Timer;

    #[test]
    fn tcp() {
        let ex = smol::LocalExecutor::new();

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        pub enum Packet {
            HandShake(HandShake),
        }
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        pub struct HandShake {
            version: u32,
        }

        async fn server() {
            let listener = net::TcpListener::bind("127.0.0.1:10870").await.unwrap();
            let stream = listener.accept().await.unwrap().0;

            let mut conn = TcpConn::<Packet>::from_tcp(&stream);

            assert_eq!(
                conn.read().await.unwrap(),
                Packet::HandShake(HandShake { version: 1 })
            );
        }
        async fn client() {
            Timer::after(Duration::from_secs(2)).await;
            let stream = net::TcpStream::connect("127.0.0.1:10870").await.unwrap();

            let mut conn = TcpConn::<Packet>::from_tcp(&stream);

            conn.send(Packet::HandShake(HandShake { version: 1 }))
                .await
                .unwrap();
        }

        ex.spawn(server()).detach();
        ex.spawn(client()).detach();

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }
    }
}
