#[cfg(feature = "mio-net")]
mod poll;

mod util;
pub use util::Resolver;

#[cfg(feature = "tcp")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "tcp")))]
pub mod tcp;

#[cfg(feature = "tcp")]
#[doc(no_inline)]
pub use tcp::{TcpListener, TcpStream};

#[cfg(feature = "udp")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "udp")))]
pub mod udp;

#[cfg(feature = "udp")]
#[doc(no_inline)]
pub use udp::UdpSocket;

#[cfg(all(unix, feature = "uds"))]
#[cfg_attr(feature = "docs", doc(cfg(all(unix, feature = "uds"))))]
pub mod uds;

#[cfg(all(unix, feature = "uds"))]
#[doc(no_inline)]
pub use uds::{UnixDatagram, UnixListener, UnixStream};
