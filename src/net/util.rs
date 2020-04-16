use crate::task::{spawn_blocking, JoinHandle};
use std::io::{self, ErrorKind::*, Result};
use std::net::{SocketAddr, ToSocketAddrs};
use std::task::Poll;

/// An exception
pub const WAKERS_LOCK_POISONED: &str = "wakers lock poisoned";

/// Async address resolver
pub trait Resolver: ToSocketAddrs {
    /// Resolve address asynchronously
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::Resolver;
    /// use std::net::SocketAddr;
    ///
    /// let addrs: Vec<SocketAddr> = "github.com:443".resolve().await?;
    /// #
    /// # Ok(()) })}
    /// ```
    fn resolve(self) -> JoinHandle<io::Result<Vec<SocketAddr>>>;
}

impl<A> Resolver for A
where
    A: 'static + Send + ToSocketAddrs,
    A::Iter: 'static + Send,
{
    #[inline]
    fn resolve(self) -> JoinHandle<io::Result<Vec<SocketAddr>>> {
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
