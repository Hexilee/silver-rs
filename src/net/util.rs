#![allow(dead_code, unused_imports)]

use crate::task::{spawn_blocking, JoinHandle};
use std::future::Future;
use std::io::{self, ErrorKind::*, Result};
use std::net::{SocketAddr, ToSocketAddrs};
use std::task::Poll;

/// An exception
pub const ENTRIES_LOCK_POISONED: &str = "entries lock poisoned";

/// Async address resolver
pub trait Resolver: ToSocketAddrs {
    /// Future to resolve address
    type ResolveFuture: Future<Output = io::Result<Vec<SocketAddr>>>;
    /// Resolve address asynchronously
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::{TcpStream, Resolver};
    ///
    /// let addrs = "github.com:443".resolve().await?;
    /// let stream = TcpStream::connect(addrs.as_slice()).await?;
    ///
    /// # Ok(()) }) }
    /// ```
    fn resolve(self) -> Self::ResolveFuture;
}

impl<A> Resolver for A
where
    A: 'static + Send + ToSocketAddrs,
    A::Iter: 'static + Send,
{
    type ResolveFuture = JoinHandle<io::Result<Vec<SocketAddr>>>;
    #[inline]
    fn resolve(self) -> Self::ResolveFuture {
        spawn_blocking(move || self.to_socket_addrs().map(Iterator::collect))
    }
}

/// Convert io result to poll
#[inline]
pub fn may_block<T>(result: Result<T>) -> Poll<Result<T>> {
    match result {
        Err(ref err) if err.kind() == WouldBlock => Poll::Pending,
        res => Poll::Ready(res),
    }
}

/// Construct invalid input io error
#[inline]
pub fn resolve_none() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "could not resolve to any valid addresses",
    )
}
