#[cfg(feature = "mio-net")]
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
