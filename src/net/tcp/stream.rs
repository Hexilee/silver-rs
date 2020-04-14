use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};
use mio::net;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct TcpStream(Arc<Watcher<net::TcpStream>>);

impl TcpStream {
    pub async fn connect(addr: SocketAddr) -> io::Result<Self> {
        let watcher = Watcher::with(net::TcpStream::connect(addr)?)?;
        let inner = Arc::new(watcher);
        futures::future::poll_fn(|cx| inner.poll_write_with(cx, |mut o| o.write(b"".as_ref())))
            .await?;
        match inner.take_error() {
            Ok(None) => Ok(Self(inner)),
            Ok(Some(err)) | Err(err) => Err(err),
        }
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_read_with(cx, |mut i| i.read(buf))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_read_with(cx, |mut i| i.read_vectored(bufs))
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_write_with(cx, |mut o| o.write(buf))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_write_with(cx, |mut o| o.write_vectored(bufs))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.poll_write_with(cx, |mut o| o.flush())
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.shutdown(std::net::Shutdown::Both)?;
        Poll::Ready(Ok(()))
    }
}
