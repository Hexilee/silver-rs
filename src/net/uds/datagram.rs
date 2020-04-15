use super::SocketAddr;
use crate::net::poll::Watcher;
use futures::future;
use mio::net;
use std::io;
use std::net::Shutdown;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::net::UnixDatagram as StdDatagram;
use std::path::Path;
use std::sync::Arc;

/// A Unix datagram socket.
///
/// After creating a `UnixDatagram` by [`bind`]ing it to a path, data can be [sent to] and
/// [received from] any other socket address.
///
/// This type is an async version of [`std::os::unix::net::UnixDatagram`].
///
/// [`std::os::unix::net::UnixDatagram`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixDatagram.html
/// [`bind`]: #method.bind
/// [received from]: #method.recv_from
/// [sent to]: #method.send_to
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use tio::net::UnixDatagram;
///
/// let socket = UnixDatagram::bind("/tmp/socket1")?;
/// socket.send_to(b"hello world", "/tmp/socket2").await?;
///
/// let mut buf = vec![0u8; 1024];
/// let (n, peer) = socket.recv_from(&mut buf).await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug, Clone)]
pub struct UnixDatagram(Arc<Watcher<net::UnixDatagram>>);

impl UnixDatagram {
    fn new(datagram: net::UnixDatagram) -> UnixDatagram {
        Self(Arc::new(Watcher::new(datagram)))
    }

    /// Creates a Unix datagram socket bound to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixDatagram> {
        let datagram = net::UnixDatagram::bind(path)?;
        Ok(UnixDatagram::new(datagram))
    }

    /// Creates a Unix datagram which is not bound to any address.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn unbound() -> io::Result<UnixDatagram> {
        let socket = net::UnixDatagram::unbound()?;
        Ok(UnixDatagram::new(socket))
    }

    /// Creates an unnamed pair of connected sockets.
    ///
    /// Returns two sockets which are connected to each other.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let (socket1, socket2) = UnixDatagram::pair()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn pair() -> io::Result<(UnixDatagram, UnixDatagram)> {
        let (a, b) = net::UnixDatagram::pair()?;
        let a = UnixDatagram::new(a);
        let b = UnixDatagram::new(b);
        Ok((a, b))
    }

    /// Connects the socket to the specified address.
    ///
    /// The [`send`] method may be used to send data to the specified address. [`recv`] and
    /// [`recv_from`] will only receive data from that address.
    ///
    /// [`send`]: #method.send
    /// [`recv`]: #method.recv
    /// [`recv_from`]: #method.recv_from
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn connect<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let p = path.as_ref();
        self.0.connect(p)
    }

    /// Returns the address of this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket")?;
    /// let addr = socket.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Returns the address of this socket's peer.
    ///
    /// The [`connect`] method will connect the socket to a peer.
    ///
    /// [`connect`]: #method.connect
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket")?;
    /// let peer = socket.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from where the data came.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.recv_from(buf)))
            .await
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket")?;
    /// let mut buf = vec![0; 1024];
    /// let n = socket.recv(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.recv(buf))).await
    }

    /// Sends data on the socket to the specified address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.send_to(b"hello world", "/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send_to<P: AsRef<Path>>(
        &self,
        buf: &[u8],
        path: P,
    ) -> io::Result<usize> {
        future::poll_fn(|cx| {
            self.0
                .poll_write_with(cx, |inner| inner.send_to(buf, path.as_ref()))
        })
        .await
    }

    /// Sends data on the socket to the socket's peer.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket")?;
    /// socket.send(b"hello world").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_write_with(cx, |inner| inner.send(buf))).await
    }

    /// Shut down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the specified portions to
    /// immediately return with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    /// use std::net::Shutdown;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }
}

impl From<StdDatagram> for UnixDatagram {
    /// Converts a `std::os::unix::net::UnixDatagram` into its asynchronous equivalent.
    fn from(datagram: StdDatagram) -> UnixDatagram {
        let mio_datagram = net::UnixDatagram::from_std(datagram);
        Self::new(mio_datagram)
    }
}

impl AsRawFd for UnixDatagram {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for UnixDatagram {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixDatagram {
        let datagram = StdDatagram::from_raw_fd(fd);
        datagram.into()
    }
}
