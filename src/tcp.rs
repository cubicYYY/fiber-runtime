//! Async TCP types backed by the [`Reactor`](crate::reactor::Reactor).
//!
//! These are thin wrappers around `mio::net` types. Each async method follows
//! the same two-step pattern:
//!
//! 1. **Try** the non-blocking syscall.
//! 2. If `WouldBlock`, **register** with the reactor and yield (`Pending`).
//!    The reactor wakes us when the OS says the socket is ready, and we retry.

use crate::reactor::{Reactor, WaitReady};
use mio::Interest;
use std::io::{self, Read, Write};
use std::net::SocketAddr;

// ---------------------------------------------------------------------------
// TcpListener
// ---------------------------------------------------------------------------

pub struct TcpListener {
    inner: mio::net::TcpListener,
    token: mio::Token,
}

impl TcpListener {
    /// Bind to `addr` and start listening (non-blocking).
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let mut inner = mio::net::TcpListener::bind(addr)?;
        let reactor = Reactor::get();
        let token = reactor.token();
        reactor.register(&mut inner, token, Interest::READABLE)?;
        Ok(Self { inner, token })
    }

    /// Accept the next incoming connection.
    ///
    /// Returns `Pending` until a client connects.
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        loop {
            match self.inner.accept() {
                Ok((stream, addr)) => return Ok((TcpStream::from_mio(stream)?, addr)),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    WaitReady::new(self.token).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        let _ = Reactor::get().deregister(&mut self.inner);
        Reactor::get().remove_waker(self.token);
    }
}

// ---------------------------------------------------------------------------
// TcpStream
// ---------------------------------------------------------------------------

pub struct TcpStream {
    inner: mio::net::TcpStream,
    token: mio::Token,
}

impl TcpStream {
    /// Wrap a `mio::net::TcpStream` (returned from `accept`) and register it
    /// with the reactor for both read and write readiness.
    fn from_mio(mut stream: mio::net::TcpStream) -> io::Result<Self> {
        let reactor = Reactor::get();
        let token = reactor.token();
        reactor.register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)?;
        Ok(Self {
            inner: stream,
            token,
        })
    }

    /// Read into `buf`, returning the number of bytes read.
    ///
    /// Returns `Ok(0)` on EOF (client disconnected).
    pub async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut stream_ref = &self.inner;
            match stream_ref.read(buf) {
                Ok(n) => return Ok(n),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    WaitReady::new(self.token).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Write `buf` to the stream, returning the number of bytes written.
    pub async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut stream_ref = &self.inner;
            match stream_ref.write(buf) {
                Ok(n) => return Ok(n),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    WaitReady::new(self.token).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Write the entire buffer, retrying partial writes.
    pub async fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        let mut written = 0;
        while written < buf.len() {
            written += self.write(&buf[written..]).await?;
        }
        Ok(())
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let _ = Reactor::get().deregister(&mut self.inner);
        Reactor::get().remove_waker(self.token);
    }
}
