use crate::net::poll::Watcher;
use crate::net::util::resolve_none;
use futures::future;
use mio::net;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket as StdSocket};
use std::sync::Arc;

/// A UDP socket.
///
/// After creating a `UdpSocket` by [`bind`]ing it to a socket address, data can be [sent to] and
/// [received from] any other socket address.
///
/// As stated in the User Datagram Protocol's specification in [IETF RFC 768], UDP is an unordered,
/// unreliable protocol. Refer to [`TcpListener`] and [`TcpStream`] for async TCP primitives.
///
/// This type is an async version of [`std::net::UdpSocket`].
///
/// [`bind`]: #method.bind
/// [received from]: #method.recv_from
/// [sent to]: #method.send_to
/// [`TcpListener`]: struct.TcpListener.html
/// [`TcpStream`]: struct.TcpStream.html
/// [`std::net`]: https://doc.rust-lang.org/std/net/index.html
/// [IETF RFC 768]: https://tools.ietf.org/html/rfc768
/// [`std::net::UdpSocket`]: https://doc.rust-lang.org/std/net/struct.UdpSocket.html
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use tio::net::UdpSocket;
///
/// let socket = UdpSocket::bind("127.0.0.1:8080")?;
/// let mut buf = vec![0u8; 1024];
///
/// loop {
///     let (n, peer) = socket.recv_from(&mut buf).await?;
///     socket.send_to(&buf[..n], &peer).await?;
/// }
/// #
/// # }) }
/// ```
#[cfg_attr(feature = "docs", doc(cfg(feature = "udp")))]
#[derive(Debug, Clone)]
pub struct UdpSocket(Arc<Watcher<net::UdpSocket>>);

impl UdpSocket {
    /// Bind a socket addr
    fn bind_once(addr: SocketAddr) -> io::Result<Self> {
        let watcher = Watcher::new(net::UdpSocket::bind(addr)?);
        let inner = Arc::new(watcher);
        match inner.take_error() {
            Ok(None) => Ok(Self(inner)),
            Ok(Some(err)) | Err(err) => Err(err),
        }
    }

    /// Creates a UDP socket from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`local_addr`] method.
    ///
    /// [`local_addr`]: #method.local_addr
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
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
    pub fn bind<A: ToSocketAddrs>(addrs: A) -> io::Result<UdpSocket> {
        let mut error = None;
        for addr in addrs.to_socket_addrs()? {
            match Self::bind_once(addr) {
                Err(err) => error = Some(err),
                ok => return ok,
            }
        }

        Err(error.unwrap_or_else(resolve_none))
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out which port was
    /// actually bound.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    ///	use tio::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
    /// let addr = socket.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Sends data on the socket to the given address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UdpSocket;
    ///
    /// const THE_MERCHANT_OF_VENICE: &[u8] = b"
    ///     If you prick us, do we not bleed?
    ///     If you tickle us, do we not laugh?
    ///     If you poison us, do we not die?
    ///     And if you wrong us, shall we not revenge?
    /// ";
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
    ///
    /// let addr = "127.0.0.1:7878";
    /// let sent = socket.send_to(THE_MERCHANT_OF_VENICE, &addr).await?;
    /// println!("Sent {} bytes to {}", sent, addr);
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
    #[inline]
    pub async fn send_to<A: ToSocketAddrs>(
        &self,
        buf: &[u8],
        addrs: A,
    ) -> io::Result<usize> {
        let addr = match addrs.to_socket_addrs()?.next() {
            Some(addr) => addr,
            None => {
                return Err(resolve_none());
            }
        };

        future::poll_fn(|cx| {
            self.0.poll_write_with(cx, |inner| inner.send_to(buf, addr))
        })
        .await
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// println!("Received {} bytes from {}", n, peer);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.recv_from(buf)))
            .await
    }

    /// Connects the UDP socket to a remote address.
    ///
    /// When connected, methods [`send`] and [`recv`] will use the specified address for sending
    /// and receiving messages. Additionally, a filter will be applied to [`recv_from`] so that it
    /// only receives messages from that same address.
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
    /// use tio::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
    /// socket.connect("127.0.0.1:8080").await?;
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
    pub async fn connect<A: ToSocketAddrs>(&self, addrs: A) -> io::Result<()> {
        let mut error = None;
        for addr in addrs.to_socket_addrs()? {
            match self.0.connect(addr) {
                Err(err) => error = Some(err),
                ok => return ok,
            }
        }
        Err(error.unwrap_or_else(resolve_none))
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The [`connect`] method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    ///
    /// [`connect`]: #method.connect
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:34254")?;
    /// socket.connect("127.0.0.1:8080").await?;
    /// let bytes = socket.send(b"Hi there!").await?;
    ///
    /// println!("Sent {} bytes", bytes);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_write_with(cx, |inner| inner.send(buf))).await
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
    /// use tio::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
    /// socket.connect("127.0.0.1:8080").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let n = socket.recv(&mut buf).await?;
    /// println!("Received {} bytes", n);
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.recv(buf))).await
    }

    /// Gets the value of the `SO_BROADCAST` option for this socket.
    ///
    /// For more information about this option, see [`set_broadcast`].
    ///
    /// [`set_broadcast`]: #method.set_broadcast
    #[inline]
    pub fn broadcast(&self) -> io::Result<bool> {
        self.0.broadcast()
    }

    /// Sets the value of the `SO_BROADCAST` option for this socket.
    ///
    /// When enabled, this socket is allowed to send packets to a broadcast address.
    #[inline]
    pub fn set_broadcast(&self, on: bool) -> io::Result<()> {
        self.0.set_broadcast(on)
    }

    /// Gets the value of the `IP_MULTICAST_LOOP` option for this socket.
    ///
    /// For more information about this option, see [`set_multicast_loop_v4`].
    ///
    /// [`set_multicast_loop_v4`]: #method.set_multicast_loop_v4
    #[inline]
    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        self.0.multicast_loop_v4()
    }

    /// Sets the value of the `IP_MULTICAST_LOOP` option for this socket.
    ///
    /// If enabled, multicast packets will be looped back to the local socket.
    ///
    /// # Note
    ///
    /// This may not have any affect on IPv6 sockets.
    #[inline]
    pub fn set_multicast_loop_v4(&self, on: bool) -> io::Result<()> {
        self.0.set_multicast_loop_v4(on)
    }

    /// Gets the value of the `IP_MULTICAST_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_multicast_ttl_v4`].
    ///
    /// [`set_multicast_ttl_v4`]: #method.set_multicast_ttl_v4
    #[inline]
    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        self.0.multicast_ttl_v4()
    }

    /// Sets the value of the `IP_MULTICAST_TTL` option for this socket.
    ///
    /// Indicates the time-to-live value of outgoing multicast packets for this socket. The default
    /// value is 1 which means that multicast packets don't leave the local network unless
    /// explicitly requested.
    ///
    /// # Note
    ///
    /// This may not have any affect on IPv6 sockets.
    #[inline]
    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> io::Result<()> {
        self.0.set_multicast_ttl_v4(ttl)
    }

    /// Gets the value of the `IPV6_MULTICAST_LOOP` option for this socket.
    ///
    /// For more information about this option, see [`set_multicast_loop_v6`].
    ///
    /// [`set_multicast_loop_v6`]: #method.set_multicast_loop_v6
    #[inline]
    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        self.0.multicast_loop_v6()
    }

    /// Sets the value of the `IPV6_MULTICAST_LOOP` option for this socket.
    ///
    /// Controls whether this socket sees the multicast packets it sends itself.
    ///
    /// # Note
    ///
    /// This may not have any affect on IPv4 sockets.
    #[inline]
    pub fn set_multicast_loop_v6(&self, on: bool) -> io::Result<()> {
        self.0.set_multicast_loop_v6(on)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`].
    ///
    /// [`set_ttl`]: #method.set_ttl
    #[inline]
    pub fn ttl(&self) -> io::Result<u32> {
        self.0.ttl()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    #[inline]
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.set_ttl(ttl)
    }

    /// Executes an operation of the `IP_ADD_MEMBERSHIP` type.
    ///
    /// This method specifies a new multicast group for this socket to join. The address must be
    /// a valid multicast address, and `interface` is the address of the local interface with which
    /// the system should join the multicast group. If it's equal to `INADDR_ANY` then an
    /// appropriate interface is chosen by the system.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use std::net::Ipv4Addr;
    ///
    /// use tio::net::UdpSocket;
    ///
    /// let interface = Ipv4Addr::new(0, 0, 0, 0);
    /// let mdns_addr = Ipv4Addr::new(224, 0, 0, 123);
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0")?;
    /// socket.join_multicast_v4(mdns_addr, interface)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn join_multicast_v4(
        &self,
        multiaddr: Ipv4Addr,
        interface: Ipv4Addr,
    ) -> io::Result<()> {
        self.0.join_multicast_v4(&multiaddr, &interface)
    }

    /// Executes an operation of the `IPV6_ADD_MEMBERSHIP` type.
    ///
    /// This method specifies a new multicast group for this socket to join. The address must be
    /// a valid multicast address, and `interface` is the index of the interface to join/leave (or
    /// 0 to indicate any interface).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use std::net::{Ipv6Addr, SocketAddr};
    ///
    /// use tio::net::UdpSocket;
    ///
    /// let socket_addr = SocketAddr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).into(), 0);
    /// let mdns_addr = Ipv6Addr::new(0xFF02, 0, 0, 0, 0, 0, 0, 0x0123);
    /// let socket = UdpSocket::bind(&socket_addr)?;
    ///
    /// socket.join_multicast_v6(&mdns_addr, 0)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub fn join_multicast_v6(
        &self,
        multiaddr: &Ipv6Addr,
        interface: u32,
    ) -> io::Result<()> {
        self.0.join_multicast_v6(multiaddr, interface)
    }

    /// Executes an operation of the `IP_DROP_MEMBERSHIP` type.
    ///
    /// For more information about this option, see [`join_multicast_v4`].
    ///
    /// [`join_multicast_v4`]: #method.join_multicast_v4
    #[inline]
    pub fn leave_multicast_v4(
        &self,
        multiaddr: Ipv4Addr,
        interface: Ipv4Addr,
    ) -> io::Result<()> {
        self.0.leave_multicast_v4(&multiaddr, &interface)
    }

    /// Executes an operation of the `IPV6_DROP_MEMBERSHIP` type.
    ///
    /// For more information about this option, see [`join_multicast_v6`].
    ///
    /// [`join_multicast_v6`]: #method.join_multicast_v6
    #[inline]
    pub fn leave_multicast_v6(
        &self,
        multiaddr: &Ipv6Addr,
        interface: u32,
    ) -> io::Result<()> {
        self.0.leave_multicast_v6(multiaddr, interface)
    }
}

impl From<StdSocket> for UdpSocket {
    fn from(socket: StdSocket) -> Self {
        let watcher = Watcher::new(net::UdpSocket::from_std(socket));
        Self(Arc::new(watcher))
    }
}
