// from https://github.com/smol-rs/smol/blob/master/examples/hyper-server.rs

//! An HTTP server based on `hyper`.
//!
//! Run with:
//!
//! ```
//! cargo run --example hyper-server
//! ```
//!
//! Open in the browser any of these addresses:
//!
//! - http://localhost:8000/

use std::net::{Shutdown, TcpListener, TcpStream};
use std::pin::Pin;
use std::task::{Context, Poll};

use smol::{io, prelude::*, Async};

/// Spawns futures.
#[derive(Clone)]
pub struct SmolExecutor;

impl<F: Future + Send + 'static> hyper::rt::Executor<F> for SmolExecutor {
    fn execute(&self, fut: F) {
        smol::spawn(async { drop(fut.await) }).detach();
    }
}

/// Listens for incoming connections.
pub struct SmolListener<'a> {
    incoming: Pin<Box<dyn Stream<Item = io::Result<Async<TcpStream>>> + Send + 'a>>,
}

impl<'a> SmolListener<'a> {
    pub fn new(listener: &'a Async<TcpListener>) -> Self {
        Self {
            incoming: Box::pin(listener.incoming()),
        }
    }
}

impl hyper::server::accept::Accept for SmolListener<'_> {
    type Conn = SmolStream;
    type Error = Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let stream = smol::ready!(self.incoming.as_mut().poll_next(cx)).unwrap()?;

        let stream = SmolStream::Plain(stream);

        Poll::Ready(Some(Ok(stream)))
    }
}

/// A TCP or TCP+TLS connection.
pub enum SmolStream {
    /// A plain TCP connection.
    Plain(Async<TcpStream>),
}

impl hyper::client::connect::Connection for SmolStream {
    fn connected(&self) -> hyper::client::connect::Connected {
        hyper::client::connect::Connected::new()
    }
}

impl tokio::io::AsyncRead for SmolStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            match &mut *self {
                SmolStream::Plain(s) => {
                    return Pin::new(s)
                        .poll_read(cx, buf.initialize_unfilled())
                        .map_ok(|size| {
                            buf.advance(size);
                        });
                }
            }
        }
    }
}

impl tokio::io::AsyncWrite for SmolStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            match &mut *self {
                SmolStream::Plain(s) => return Pin::new(s).poll_write(cx, buf),
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            SmolStream::Plain(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            SmolStream::Plain(s) => {
                s.get_ref().shutdown(Shutdown::Write)?;
                Poll::Ready(Ok(()))
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Client may have never connected")]
    IoError(#[from] io::Error),
    #[error("Client may have never connected")]
    HyperError(#[from] hyper::Error),
}
