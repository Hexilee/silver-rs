use crate::net::poll::{Watcher, REACTOR};
use crate::net::util::may_block;
use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};
use mio::net;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct TcpStream(Arc<Watcher<net::TcpStream>>);

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let watcher = &*self.0;
        REACTOR.read(watcher.token, cx.waker().clone());
        may_block((&mut &watcher.source).read(buf))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        let watcher = &*self.0;
        REACTOR.read(watcher.token, cx.waker().clone());
        may_block((&mut &watcher.source).read_vectored(bufs))
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let watcher = &*self.0;
        REACTOR.write(watcher.token, cx.waker().clone());
        may_block((&mut &watcher.source).write(buf))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let watcher = &*self.0;
        REACTOR.write(watcher.token, cx.waker().clone());
        may_block((&mut &watcher.source).write_vectored(bufs))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let watcher = &*self.0;
        REACTOR.write(watcher.token, cx.waker().clone());
        may_block((&mut &watcher.source).flush())
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.shutdown(std::net::Shutdown::Write)?;
        Poll::Ready(Ok(()))
    }
}
