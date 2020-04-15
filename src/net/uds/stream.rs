use super::SocketAddr;
use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};
use mio::net;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::net::Shutdown;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::net::UnixStream as StdStream;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

/// A Unix stream socket.
///
/// This type is an async version of [`std::os::unix::net::UnixStream`].
///
/// [`std::os::unix::net::UnixStream`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use tio::net::UnixStream;
/// use futures::prelude::*;
///
/// let mut stream = UnixStream::connect("/tmp/socket")?;
/// stream.write_all(b"hello world").await?;
///
/// let mut response = Vec::new();
/// stream.read_to_end(&mut response).await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug, Clone)]
pub struct UnixStream(pub(crate) Arc<Watcher<net::UnixStream>>);

impl UnixStream {
    /// Connects to the socket to the specified address.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        let path = path.as_ref().to_owned();
        let mio_stream = net::UnixStream::connect(path)?;
        Ok(UnixStream(Arc::new(Watcher::new(mio_stream))))
    }

    /// Creates an unnamed pair of connected sockets.
    ///
    /// Returns two streams which are connected to each other.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixStream;
    ///
    /// let stream = UnixStream::pair()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn pair() -> io::Result<(UnixStream, UnixStream)> {
        let (a, b) = net::UnixStream::pair()?;
        let a = UnixStream(Arc::new(Watcher::new(a)));
        let b = UnixStream(Arc::new(Watcher::new(b)));
        Ok((a, b))
    }

    /// Returns the socket address of the local half of this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket")?;
    /// let addr = stream.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Returns the socket address of the remote half of this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket")?;
    /// let peer = stream.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the specified portions to
    /// immediately return with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixStream;
    /// use std::net::Shutdown;
    ///
    /// let stream = UnixStream::connect("/tmp/socket")?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }
}

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
