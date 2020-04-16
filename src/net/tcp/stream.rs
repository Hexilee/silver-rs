use crate::net::poll::Watcher;
use crate::net::util::resolve_none;
use futures::task::{Context, Poll};
use futures::{future, AsyncRead, AsyncWrite};
use mio::net;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::net::{SocketAddr, TcpStream as StdStream, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;

/// A TCP stream between a local and a remote socket.
///
/// A `TcpStream` can either be created by connecting to an endpoint, via the [`connect`] method,
/// or by [accepting] a connection from a [listener].  It can be read or written to using the
/// [`AsyncRead`], [`AsyncWrite`], and related extension traits in [`futures::io`].
///
/// The connection will be closed when the value is dropped. The reading and writing portions of
/// the connection can also be shut down individually with the [`shutdown`] method.
///
/// This type is an async version of [`std::net::TcpStream`].
///
/// [`connect`]: struct.TcpStream.html#method.connect
/// [accepting]: struct.TcpListener.html#method.accept
/// [listener]: struct.TcpListener.html
/// [`AsyncRead`]: https://docs.rs/futures/0.3/futures/io/trait.AsyncRead.html
/// [`AsyncWrite`]: https://docs.rs/futures/0.3/futures/io/trait.AsyncWrite.html
/// [`futures::io`]: https://docs.rs/futures/0.3/futures/io/index.html
/// [`shutdown`]: struct.TcpStream.html#method.shutdown
/// [`std::net::TcpStream`]: https://doc.rust-lang.org/std/net/struct.TcpStream.html
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use tio::net::TcpStream;
/// use futures::prelude::*;
///
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
/// stream.write_all(b"hello world").await?;
///
/// let mut buf = vec![0u8; 1024];
/// let n = stream.read(&mut buf).await?;
/// #
/// # Ok(()) }) }
/// ```
#[cfg_attr(feature = "docs", doc(cfg(feature = "tcp")))]
#[derive(Debug, Clone)]
pub struct TcpStream(pub(crate) Arc<Watcher<net::TcpStream>>);

impl TcpStream {
    /// Connect to a socket addr
    async fn connect_once(addr: SocketAddr) -> io::Result<Self> {
        let watcher = Watcher::new(net::TcpStream::connect(addr)?);
        // wait for connection established
        watcher.write_ready().await;
        let inner = Arc::new(watcher);
        match inner.take_error() {
            Ok(None) => Ok(Self(inner)),
            Ok(Some(err)) | Err(err) => Err(err),
        }
    }

    /// Creates a new TCP stream connected to the specified address.
    ///
    /// This method will create a new TCP socket and attempt to connect it to the `addr`
    /// provided. The returned future will be resolved once the stream has successfully
    /// connected, or it will return an error if one occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    ///
    /// # Blocking
    ///
    /// This method may be blocked by resolving.
    /// You can resolve addrs asynchronously by [`Resolver`].
    ///
    /// [`Resolver`]: trait.Resolver.html
    pub async fn connect(addrs: impl ToSocketAddrs) -> io::Result<Self> {
        let mut error = None;
        for addr in addrs.to_socket_addrs()? {
            match Self::connect_once(addr).await {
                Err(err) => {
                    error = Some(err);
                }
                ok => return ok,
            }
        }
        Err(error.unwrap_or_else(resolve_none))
    }
    /// Returns the local address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let addr = stream.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Returns the remote address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let peer = stream.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`].
    ///
    /// [`set_ttl`]: #method.set_ttl
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn ttl(&self) -> io::Result<u32> {
        self.0.ttl()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.set_ttl(ttl)
    }

    /// Receives data on the socket from the remote address to which it is connected, without
    /// removing that data from the queue.
    ///
    /// On success, returns the number of bytes peeked.
    ///
    /// Successive calls return the same data. This is accomplished by passing `MSG_PEEK` as a flag
    /// to the underlying `recv` system call.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let n = stream.peek(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.peek(buf))).await
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// For more information about this option, see [`set_nodelay`].
    ///
    /// [`set_nodelay`]: #method.set_nodelay
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn nodelay(&self) -> io::Result<bool> {
        self.0.nodelay()
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that
    /// segments are always sent as soon as possible, even if there is only a
    /// small amount of data. When not set, data is buffered until there is a
    /// sufficient amount to send out, thereby avoiding the frequent sending of
    /// small packets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.0.set_nodelay(nodelay)
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This method will cause all pending and future I/O on the specified portions to return
    /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use std::net::Shutdown;
    ///
    /// use tio::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.0.shutdown(how)
    }
}

impl From<StdStream> for TcpStream {
    fn from(stream: StdStream) -> Self {
        let watcher = Watcher::new(net::TcpStream::from_std(stream));
        Self(Arc::new(watcher))
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
        self.shutdown(std::net::Shutdown::Both)?;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::TcpStream;
    use crate::task::block_on;
    use futures::{AsyncReadExt, AsyncWriteExt};
    use std::io;
    use std::net::{Shutdown, SocketAddr, TcpListener};
    use std::thread;

    const DATA: &[u8] = b"
    If you prick us, do we not bleed?
    If you tickle us, do we not laugh?
    If you poison us, do we not die?
    And if you wrong us, shall we not revenge?
    ";

    fn start_server(check: bool) -> io::Result<SocketAddr> {
        use std::io::{Read, Write};
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        if check {
            thread::spawn(move || {
                let mut data = [0; DATA.len()];
                while let Ok((mut stream, addr)) = listener.accept() {
                    stream.read_exact(data.as_mut()).unwrap();
                    assert_eq!(DATA, data.as_ref());
                    stream.write_all(addr.to_string().as_bytes()).unwrap();
                }
            });
        } else {
            thread::spawn(move || {
                let mut data = [0; 1024];
                let (mut stream, _) = listener.accept().unwrap();
                while let Ok(_) = stream.read(&mut data) {
                    // do nothing...
                }
            });
        }
        Ok(addr)
    }

    #[test]
    fn stream_async() -> io::Result<()> {
        block_on(async {
            let addr = start_server(true)?;
            let mut stream = TcpStream::connect(addr).await?;
            stream.write_all(DATA).await?;

            let mut recv_data = String::new();
            stream.read_to_string(&mut recv_data).await?;
            let local_addr: SocketAddr = recv_data.parse().unwrap();
            Ok(assert_eq!(local_addr, stream.local_addr()?))
        })
    }

    #[test]
    fn from_std() -> io::Result<()> {
        let addr = start_server(true)?;
        let raw_stream = std::net::TcpStream::connect(addr)?;
        block_on(async move {
            let mut stream = TcpStream::from(raw_stream);
            stream.write_all(DATA).await?;

            let mut recv_data = String::new();
            stream.read_to_string(&mut recv_data).await?;
            let local_addr: SocketAddr = recv_data.parse().unwrap();
            Ok(assert_eq!(local_addr, stream.local_addr()?))
        })
    }

    #[test]
    fn peek() -> io::Result<()> {
        async fn peek_to_string(stream: TcpStream) -> io::Result<String> {
            let mut data = [0; 1024];
            let size = stream.peek(&mut data).await?;
            Ok(String::from_utf8(data[..size].to_vec()).unwrap())
        }

        block_on(async {
            let addr = start_server(true)?;
            let mut stream = TcpStream::connect(addr).await?;
            stream.write_all(DATA).await?;
            let data1 = peek_to_string(stream.clone()).await?;
            let data2 = peek_to_string(stream.clone()).await?;
            let local_addr: SocketAddr = data1.parse().unwrap();
            assert_eq!(data1, data2);
            Ok(assert_eq!(local_addr, stream.local_addr()?))
        })
    }

    #[test]
    fn peer_addr() -> io::Result<()> {
        block_on(async {
            let addr = start_server(false)?;
            let stream = TcpStream::connect(addr).await?;
            let peer_addr = stream.peer_addr()?;
            Ok(assert_eq!(addr, peer_addr))
        })
    }
    #[test]
    fn ttl() -> io::Result<()> {
        block_on(async {
            let addr = start_server(false)?;
            let stream = TcpStream::connect(addr).await?;
            stream.set_ttl(100)?;
            Ok(assert_eq!(100, stream.ttl()?))
        })
    }
    #[test]
    fn no_delay() -> io::Result<()> {
        block_on(async {
            let addr = start_server(false)?;
            let stream = TcpStream::connect(addr).await?;
            let no_delay = stream.nodelay()?;
            stream.set_nodelay(!no_delay)?;
            Ok(assert_eq!(!no_delay, stream.nodelay()?))
        })
    }

    #[test]
    fn shutdown() -> io::Result<()> {
        block_on(async {
            let addr = start_server(false)?;
            let mut stream = TcpStream::connect(addr).await?;
            stream.shutdown(Shutdown::Write)?;
            Ok(assert!(stream.write_all(DATA).await.is_err()))
        })
    }
}
