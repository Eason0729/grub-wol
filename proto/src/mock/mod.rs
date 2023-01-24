// edit from https://hackmd.io/@lbernick/SkgO7bCMw
use std::io;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{self, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite};

pub struct MockTcpStream {
    writer: Option<Arc<Mutex<VecDeque<u8>>>>,
    reader: Option<Arc<Mutex<VecDeque<u8>>>>,
}

impl AsyncWrite for MockTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match &self.writer {
            Some(writer) => {
                let mut writer = writer.lock().unwrap();
                let size = io::Write::write(&mut *writer, buf).unwrap();
                Poll::Ready(Ok(size))
            }
            None => Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.reader = None;
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for MockTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match &self.reader {
            Some(reader) => {
                let mut reader = reader.lock().unwrap();
                let mut slice=vec![0;1024];
                let size = io::Read::read(&mut *reader, &mut slice).unwrap();
                buf.put_slice(&slice[0..size]);
                Poll::Ready(Ok(()))
            }
            None => Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
        }
    }
}

impl MockTcpStream {
    pub fn new_pair() -> (Self, Self) {
        let rw = Some(Arc::new(Mutex::new(VecDeque::new())));
        let wr = Some(Arc::new(Mutex::new(VecDeque::new())));

        let rs = MockTcpStream {
            writer: rw.clone(),
            reader: wr.clone(),
        };
        let ws = MockTcpStream {
            writer: wr,
            reader: rw,
        };
        (rs, ws)
    }
}
