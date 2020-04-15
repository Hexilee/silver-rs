use super::{SocketAddr, UnixStream};
use crate::net::poll::Watcher;
use futures::task::{Context, Poll};
use futures::{future, Stream};
use mio::net;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
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
            future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.accept())).await?;
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
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (io, _) = futures::ready!(self.0.poll_read_with(cx, |inner| inner.accept()))?;
        let stream = UnixStream(Arc::new(Watcher::new(io)));
        Poll::Ready(Some(Ok(stream)))
    }
}

impl From<StdListener> for UnixListener {
    /// Converts a `std::os::unix::net::UnixListener` into its asynchronous equivalent.
    fn from(listener: StdListener) -> UnixListener {
        let mio_listener = net::UnixListener::from_std(listener);
        UnixListener(Arc::new(Watcher::new(mio_listener)))
    }
}

impl AsRawFd for UnixListener {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for UnixListener {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixListener {
        let listener = StdListener::from_raw_fd(fd);
        listener.into()
    }
}
