// edit from https://hackmd.io/@lbernick/SkgO7bCMw
use async_std::io::{Read, Write};
use std::io;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{self, Poll},
};

pub struct MockTcpStream {
    writer: Option<Arc<Mutex<VecDeque<u8>>>>,
    reader: Option<Arc<Mutex<VecDeque<u8>>>>,
}

impl Write for MockTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &self.writer {
            Some(writer) => {
                let mut writer = writer.lock().unwrap();
                let size = io::Write::write(&mut *writer, buf).unwrap();
                Poll::Ready(Ok(size))
            }
            None => Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<io::Result<()>> {
        self.reader = None;
        Poll::Ready(Ok(()))
    }
}

impl Read for MockTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match &self.reader {
            Some(reader) => {
                let mut reader = reader.lock().unwrap();
                let size = io::Read::read(&mut *reader, buf).unwrap();
                Poll::Ready(Ok(size))
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
