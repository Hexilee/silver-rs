mod poll;
mod util;

#[cfg(feature = "tcp")]
mod tcp;

#[cfg(feature = "tcp")]
pub use tcp::{TcpListener, TcpStream};

#[cfg(feature = "udp")]
mod udp;

#[cfg(feature = "udp")]
pub use udp::UdpSocket;

#[cfg(all(unix, feature = "uds"))]
pub mod uds;

#[cfg(all(unix, feature = "uds"))]
pub use uds::{UnixDatagram, UnixListener, UnixStream};

pub use util::Resolver;
