//! Networking primitives for TCP/UDP communication.
//!
//! This module provides networking functionality for the Transmission Control and User
//! Datagram Protocols.
//!
//!
//! # Organization
//!
//! * [`TcpListener`] and [`TcpStream`] provide functionality for communication over TCP
//! * [`UdpSocket`] provides functionality for communication over UDP
//! * [`Resolver`] provides functionality to asynchronously resolve socket address
//!
//!
//! [`TcpListener`]: struct.TcpListener.html
//! [`TcpStream`]: struct.TcpStream.html
//! [`UdpSocket`]: struct.UdpSocket.html
//! [`Resolver`]: trait.Resolver.html
//!
//! # Platform-specific extensions
//!
//! APIs such as Unix domain sockets are available on certain platforms only. You can find
//! platform-specific extensions in the [`uds`] submodule.
//!
//! [`uds`]: uds/index.html
//!
//! # Examples
//!
//! A simple UDP echo server:
//!
//! ```no_run
//! # fn main() -> std::io::Result<()> { tio::task::block_on(async {
//! #
//! use tio::net::UdpSocket;
//!
//! let socket = UdpSocket::bind("127.0.0.1:8080")?;
//! let mut buf = vec![0u8; 1024];
//!
//! loop {
//!     let (n, peer) = socket.recv_from(&mut buf).await?;
//!     socket.send_to(&buf[..n], &peer).await?;
//! }
//! #
//! # }) }
//! ```

#[cfg(feature = "event-loop")]
mod poll;

mod util;
pub use util::Resolver;

#[cfg(feature = "tcp")]
mod tcp;

#[cfg(feature = "tcp")]
pub use tcp::{TcpListener, TcpStream};

#[cfg(feature = "udp")]
mod udp;

#[cfg(feature = "udp")]
pub use udp::UdpSocket;

#[cfg(all(unix, feature = "uds"))]
#[cfg_attr(feature = "docs", doc(cfg(all(unix, feature = "uds"))))]
pub mod uds;

#[cfg(all(unix, feature = "uds"))]
#[doc(no_inline)]
pub use uds::{UnixDatagram, UnixListener, UnixStream};
