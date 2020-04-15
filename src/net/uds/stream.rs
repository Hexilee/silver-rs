use super::SocketAddr;
use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{future, AsyncRead, AsyncWrite};
use mio::net;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::net::UnixStream as StdStream;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct UnixStream(pub(crate) Arc<Watcher<net::UnixStream>>);

impl UnixStream {}

impl From<StdStream> for UnixStream {
    fn from(stream: StdStream) -> Self {
        let watcher = Watcher::new(net::UnixStream::from_std(stream));
        Self(Arc::new(watcher))
    }
}

impl AsyncRead for UnixStream {
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

impl AsyncWrite for UnixStream {
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

impl Read for UnixStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&mut &**self.0).read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        (&mut &**self.0).read_vectored(bufs)
    }
}

impl Write for UnixStream {
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

impl AsRawFd for UnixStream {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for UnixStream {
    /// Converts a `RawFd` to a `UnixStream`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the socket is in
    /// non-blocking mode.
    unsafe fn from_raw_fd(fd: RawFd) -> UnixStream {
        let std_stream: StdStream = FromRawFd::from_raw_fd(fd);
        std_stream.into()
    }
}
