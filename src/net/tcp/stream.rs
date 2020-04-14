use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};
use mio::net;
use std::convert::TryFrom;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::net::{SocketAddr, TcpStream as StdStream};
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

impl TryFrom<StdStream> for TcpStream {
    type Error = io::Error;
    fn try_from(stream: StdStream) -> Result<Self, Self::Error> {
        let watcher = Watcher::with(net::TcpStream::from_std(stream))?;
        Ok(Self(Arc::new(watcher)))
    }
}

impl AsyncRead for TcpStream {
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_read_with(cx, |mut i| i.read(buf))
    }

    #[inline]
    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_read_with(cx, |mut i| i.read_vectored(bufs))
    }
}

impl AsyncWrite for TcpStream {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_write_with(cx, |mut o| o.write(buf))
    }

    #[inline]
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_write_with(cx, |mut o| o.write_vectored(bufs))
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.poll_write_with(cx, |mut o| o.flush())
    }

    #[inline]
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.shutdown(std::net::Shutdown::Both)?;
        Poll::Ready(Ok(()))
    }
}

impl Read for TcpStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&mut &**self.0).read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        (&mut &**self.0).read_vectored(bufs)
    }
}

impl Write for TcpStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&mut &**self.0).write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        (&mut &**self.0).write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        (&mut &**self.0).flush()
    }
}
