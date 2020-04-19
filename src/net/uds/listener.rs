use super::{SocketAddr, UnixStream};
use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{future, Stream};
use mio::net;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixListener as StdListener;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

/// A Unix domain socket server, listening for connections.
///
/// After creating a `UnixListener` by [`bind`]ing it to a socket address, it listens for incoming
/// connections. These can be accepted by awaiting elements from the async stream of [`incoming`]
/// connections.
///
/// The socket will be closed when the value is dropped.
///
/// This type is an async version of [`std::os::unix::net::UnixListener`].
///
/// [`std::os::unix::net::UnixListener`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixListener.html
/// [`bind`]: #method.bind
/// [`incoming`]: #method.incoming
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use tio::net::UnixListener;
/// use futures::prelude::*;
///
/// let mut listener = UnixListener::bind("/tmp/socket")?;
///
/// while let Some(stream) = listener.next().await {
///     let mut stream = stream?;
///     stream.write_all(b"hello world").await?;
/// }
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug, Clone)]
pub struct UnixListener(Arc<Watcher<net::UnixListener>>);

impl UnixListener {
    /// Creates a Unix datagram listener bound to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixListener;
    ///
    /// let listener = UnixListener::bind("/tmp/socket")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
        let listener = net::UnixListener::bind(path)?;
        Ok(UnixListener(Arc::new(Watcher::new(listener))))
    }

    /// Accepts a new incoming connection to this listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixListener;
    ///
    /// let listener = UnixListener::bind("/tmp/socket")?;
    /// let (socket, addr) = listener.accept().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
        let (io, addr) =
            future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.accept()))
                .await?;
        let stream = UnixStream(Arc::new(Watcher::new(io)));
        Ok((stream, addr))
    }

    /// Returns the local socket address of this listener.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixListener;
    ///
    /// let listener = UnixListener::bind("/tmp/socket")?;
    /// let addr = listener.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl Stream for UnixListener {
    type Item = io::Result<UnixStream>;

    /// Returns a stream of incoming connections.
    ///
    /// Iterating over this stream is equivalent to calling [`accept`] in a loop. The stream of
    /// connections is infinite, i.e awaiting the next connection will never result in [`None`].
    ///
    /// [`accept`]: #method.accept
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixListener;
    /// use futures::prelude::*;
    ///
    /// let mut listener = UnixListener::bind("/tmp/socket")?;
    ///
    /// while let Some(stream) = listener.next().await {
    ///     let mut stream = stream?;
    ///     stream.write_all(b"hello world").await?;
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let (io, _) =
            futures::ready!(self.0.poll_read_with(cx, |inner| inner.accept()))?;
        let stream = UnixStream(Arc::new(Watcher::new(io)));
        Poll::Ready(Some(Ok(stream)))
    }
}

impl From<StdListener> for UnixListener {
    /// Converts a `std::os::unix::net::UnixListener` into its asynchronous equivalent.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the listener is in
    /// non-blocking mode.
    fn from(listener: StdListener) -> UnixListener {
        let mio_listener = net::UnixListener::from_std(listener);
        UnixListener(Arc::new(Watcher::new(mio_listener)))
    }
}

impl AsRawFd for UnixListener {
    /// Share raw fd of `UnixListener`.
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
    use crate::net::uds::{UnixListener, UnixStream};
    use crate::task::{block_on, sleep, spawn};
    use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
    use std::io;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use std::path::{Path, PathBuf};
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

    async fn connect(path: impl AsRef<Path>) -> io::Result<()> {
        sleep(Duration::from_secs(1)).await;
        let mut stream = UnixStream::connect(path).await?;
        stream.write_all(DATA).await?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data).await?;
        Ok(assert_eq!(DATA, data.as_slice()))
    }

    #[test]
    fn accept() -> io::Result<()> {
        block_on(async {
            let path_buf = random_path()?;
            let listener = UnixListener::bind(path_buf.as_path())?;
            spawn(async move {
                let mut data = [0; DATA.len()];
                while let Ok((mut stream, addr)) = listener.accept().await {
                    assert!(addr.is_unnamed());
                    stream.read_exact(&mut data).await.unwrap();
                    assert_eq!(DATA, data.as_ref());
                    stream.write_all(&data).await.unwrap();
                }
            });
            connect(path_buf).await
        })
    }

    #[test]
    fn from_std() -> io::Result<()> {
        block_on(async {
            let path_buf = random_path()?;
            let raw_listener =
                std::os::unix::net::UnixListener::bind(path_buf.as_path())?;
            raw_listener.set_nonblocking(true)?;
            spawn(async move {
                let listener = UnixListener::from(raw_listener);
                let mut data = [0; DATA.len()];
                while let Ok((mut stream, addr)) = listener.accept().await {
                    assert!(addr.is_unnamed());
                    stream.read_exact(&mut data).await.unwrap();
                    assert_eq!(DATA, data.as_ref());
                    stream.write_all(&data).await.unwrap();
                }
            });
            connect(path_buf).await
        })
    }

    #[test]
    fn as_raw_fd() -> io::Result<()> {
        use std::io::{Read, Write};
        block_on(async {
            let path_buf = random_path()?;
            let listener = UnixListener::bind(path_buf.as_path())?;
            let raw_fd = listener.as_raw_fd();
            spawn(async move {
                let raw_listener =
                    unsafe { std::os::unix::net::UnixListener::from_raw_fd(raw_fd) };
                raw_listener.set_nonblocking(false).unwrap();
                let mut data = [0; DATA.len()];
                while let Ok((mut stream, addr)) = raw_listener.accept() {
                    assert!(addr.is_unnamed());
                    stream.read_exact(&mut data).unwrap();
                    assert_eq!(DATA, data.as_ref());
                    stream.write_all(&data).unwrap();
                }
                drop(listener); // drop stream before raw_listener is dropped and fd is closed
            });
            connect(path_buf).await
        })
    }

    #[test]
    fn stream() -> io::Result<()> {
        block_on(async {
            let path_buf = random_path()?;
            let mut listener = UnixListener::bind(path_buf.as_path())?;
            spawn(async move {
                let mut data = [0; DATA.len()];
                while let Some(Ok(mut stream)) = listener.next().await {
                    stream.read_exact(&mut data).await.unwrap();
                    assert_eq!(DATA, data.as_ref());
                    stream.write_all(&data).await.unwrap();
                }
            });
            connect(path_buf).await
        })
    }

    #[test]
    fn local_addr() -> io::Result<()> {
        let path_buf = random_path()?;
        let listener = UnixListener::bind(path_buf.as_path())?;
        Ok(assert_eq!(
            Some(path_buf.as_path()),
            listener.local_addr()?.as_pathname()
        ))
    }
}
