use super::TcpStream;
use crate::net::poll::Watcher;
use crate::net::util::resolve_none;
use futures::task::{Context, Poll};
use futures::{future, Stream};
use mio::net;
use std::io;
use std::net::{SocketAddr, TcpListener as StdListener, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;

/// A TCP socket server, listening for connections.
///
/// After creating a `TcpListener` by [`bind`]ing it to a socket address, it listens for incoming
/// TCP connections. These can be accepted by awaiting elements from the async stream of
/// [`incoming`] connections.
///
/// The socket will be closed when the value is dropped.
///
/// The Transmission Control Protocol is specified in [IETF RFC 793].
///
/// This type is an async version of [`std::net::TcpListener`].
///
/// [`bind`]: #method.bind
/// [`incoming`]: #method.incoming
/// [IETF RFC 793]: https://tools.ietf.org/html/rfc793
/// [`std::net::TcpListener`]: https://doc.rust-lang.org/std/net/struct.TcpListener.html
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use futures::io;
/// use futures::prelude::*;
/// use tio::net::TcpListener;
///
/// let mut listener = TcpListener::bind("127.0.0.1:8080")?;
///
/// while let Some(stream) = listener.next().await {
///     let stream = stream?;
///     let read_stream = stream.clone();
///     let (reader, writer) = &mut (read_stream, stream);
///     io::copy(reader, writer).await?;
/// }
/// #
/// # Ok(()) }) }
/// ```
#[cfg_attr(feature = "docs", doc(cfg(feature = "tcp")))]
#[derive(Debug, Clone)]
pub struct TcpListener(Arc<Watcher<net::TcpListener>>);

impl TcpListener {
    /// Bind a socket addr
    fn bind_once(addr: SocketAddr) -> io::Result<Self> {
        let watcher = Watcher::new(net::TcpListener::bind(addr)?);
        let inner = Arc::new(watcher);
        match inner.take_error() {
            Ok(None) => Ok(Self(inner)),
            Ok(Some(err)) | Err(err) => Err(err),
        }
    }

    /// Creates a new `TcpListener` which will be bound to the specified address.
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the [`local_addr`] method.
    ///
    /// # Examples
    /// Create a TCP listener bound to 127.0.0.1:0:
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    ///
    /// [`local_addr`]: #method.local_addr
    pub fn bind<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpListener> {
        let mut error = None;

        for addr in addrs.to_socket_addrs()? {
            match Self::bind_once(addr) {
                Err(err) => error = Some(err),
                ok => return ok,
            }
        }

        Err(error.unwrap_or_else(resolve_none))
    }

    /// Accepts a new incoming connection to this listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0")?;
    /// let (stream, addr) = listener.accept().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (io, addr) =
            future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.accept()))
                .await?;

        let stream = TcpStream(Arc::new(Watcher::new(io)));
        Ok((stream, addr))
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, to identify when binding to port 0 which port was assigned
    /// by the OS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:8080")?;
    /// let addr = listener.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl Stream for TcpListener {
    type Item = io::Result<TcpStream>;

    /// Iterating over this stream is equivalent to calling [`accept`] in a loop. The stream of
    /// connections is infinite, i.e awaiting the next connection will never result in [`None`].
    ///
    /// [`accept`]: #method.accept
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpListener;
    /// use futures::prelude::*;
    ///
    /// let mut listener = TcpListener::bind("127.0.0.1:0")?;
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
        let stream = TcpStream(Arc::new(Watcher::new(io)));
        Poll::Ready(Some(Ok(stream)))
    }
}

impl From<StdListener> for TcpListener {
    fn from(listener: StdListener) -> Self {
        let watcher = Watcher::new(net::TcpListener::from_std(listener));
        Self(Arc::new(watcher))
    }
}
