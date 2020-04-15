use crate::task::{spawn_blocking, JoinHandle};
use std::io::{self, ErrorKind::*, Result};
use std::net::{SocketAddr, ToSocketAddrs};
use std::task::Poll;

pub trait Resolver: ToSocketAddrs {
    fn resolve(self) -> JoinHandle<io::Result<Vec<SocketAddr>>>;
}

impl<A> Resolver for A
where
    A: 'static + Send + ToSocketAddrs,
    A::Iter: 'static + Send,
{
    fn resolve(self) -> JoinHandle<io::Result<Vec<SocketAddr>>> {
        spawn_blocking(move || self.to_socket_addrs().map(Iterator::collect))
    }
}

#[inline]
pub fn may_block<T>(result: Result<T>) -> Poll<Result<T>> {
    match result {
        Err(ref err) if err.kind() == WouldBlock => Poll::Pending,
        res => Poll::Ready(res),
    }
}
