use super::SocketAddr;
use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};
use mio::net;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::net::Shutdown;
use std::os::unix::io::{AsRawFd, RawFd};
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
/// let mut stream = UnixStream::connect("/tmp/socket").await?;
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
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        let watcher = Watcher::new(net::UnixStream::connect(path)?);
        watcher.write_ready().await;
        let inner = Arc::new(watcher);
        match inner.take_error() {
            Ok(None) => Ok(Self(inner)),
            Ok(Some(err)) | Err(err) => Err(err),
        }
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
    /// let stream = UnixStream::connect("/tmp/socket").await?;
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
    /// let stream = UnixStream::connect("/tmp/socket").await?;
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
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }
}

impl From<StdStream> for UnixStream {
    /// Creates a new `UnixStream` from a standard `net::UnixStream`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the socket is in
    /// non-blocking mode.
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

impl AsRawFd for UnixStream {
    /// Share raw fd of `UnixStream`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for never closing this fd.
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::UnixStream;
    use crate::task::block_on;
    use futures::{AsyncReadExt, AsyncWriteExt};
    use std::io;
    use std::net::Shutdown;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    const DATA: &[u8] = b"
    If you prick us, do we not bleed?
    If you tickle us, do we not laugh?
    If you poison us, do we not die?
    And if you wrong us, shall we not revenge?
    ";

    fn random_path() -> io::Result<PathBuf> {
        Ok(NamedTempFile::new()?.path().to_path_buf())
    }

    fn start_server() -> io::Result<PathBuf> {
        use std::io::{Read, Write};
        use std::thread::{sleep, spawn};

        let path_buf = random_path()?;
        let listener = std::os::unix::net::UnixListener::bind(path_buf.as_path())?;
        spawn(move || {
            let mut data = [0; DATA.len()];
            while let Ok((mut stream, addr)) = listener.accept() {
                assert!(addr.is_unnamed());
                stream.read_exact(&mut data).unwrap();
                assert_eq!(DATA, data.as_ref());
                stream.write_all(&data).unwrap();
            }
        });
        sleep(Duration::from_secs(1));
        Ok(path_buf)
    }

    #[test]
    fn stream() -> io::Result<()> {
        block_on(async {
            let addr = start_server()?;
            let mut stream = UnixStream::connect(addr).await?;
            assert!(stream.local_addr()?.is_unnamed());
            stream.write_all(DATA).await?;

            let mut data = Vec::new();
            stream.read_to_end(&mut data).await?;
            assert_eq!(DATA, data.as_slice());
            Ok(())
        })
    }

    #[test]
    fn from_std() -> io::Result<()> {
        block_on(async move {
            let addr = start_server()?;
            let raw_stream = std::os::unix::net::UnixStream::connect(addr)?;
            raw_stream.set_nonblocking(true)?;
            let mut stream = UnixStream::from(raw_stream);
            assert!(stream.local_addr()?.is_unnamed());
            stream.write_all(DATA).await?;

            let mut data = Vec::new();
            stream.read_to_end(&mut data).await?;
            assert_eq!(DATA, data.as_slice());
            Ok(())
        })
    }

    #[test]
    fn as_raw_fd() -> io::Result<()> {
        use std::io::{Read, Write};
        block_on(async move {
            let addr = start_server()?;
            let stream = UnixStream::connect(addr).await?;
            let raw_fd = stream.as_raw_fd();
            let mut raw_stream =
                unsafe { std::os::unix::net::UnixStream::from_raw_fd(raw_fd) };
            raw_stream.set_nonblocking(false)?;
            assert!(raw_stream.local_addr()?.is_unnamed());
            raw_stream.write_all(DATA)?;

            let mut data = Vec::new();
            raw_stream.read_to_end(&mut data)?;

            drop(stream); // drop stream before raw_stream is dropped and fd is closed
            assert_eq!(DATA, data.as_slice());
            Ok(())
        })
    }

    #[test]
    fn peer_addr() -> io::Result<()> {
        block_on(async {
            let path_buf = random_path()?;
            let path = path_buf.as_path();
            let _listener = std::os::unix::net::UnixListener::bind(path)?;
            let stream = UnixStream::connect(path).await?;
            let peer_addr = stream.peer_addr()?;
            assert_eq!(Some(path), peer_addr.as_pathname());
            Ok(())
        })
    }

    #[test]
    fn shutdown() -> io::Result<()> {
        block_on(async {
            let path_buf = random_path()?;
            let path = path_buf.as_path();
            let _listener = std::os::unix::net::UnixListener::bind(path.to_path_buf())?;
            let mut stream = UnixStream::connect(path).await?;
            stream.shutdown(Shutdown::Write)?;
            assert!(stream.write_all(DATA).await.is_err());
            Ok(())
        })
    }

    #[test]
    fn pair() -> io::Result<()> {
        use crate::task::spawn;
        block_on(async {
            let (mut s1, mut s2) = UnixStream::pair()?;
            spawn(async move {
                let mut data = [0; DATA.len()];
                s2.read_exact(&mut data).await.unwrap();
                assert_eq!(DATA, data.as_ref());
                s2.write_all(&data).await.unwrap();
            });
            s1.write_all(DATA).await?;
            let mut data = Vec::new();
            s1.read_to_end(&mut data).await?;
            assert_eq!(DATA, data.as_slice());
            Ok(())
        })
    }
}
