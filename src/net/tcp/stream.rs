use crate::net::poll::Watcher;
use mio::net;
use std::sync::Arc;

#[derive(Clone)]
pub struct TcpStream(Arc<Watcher<net::TcpStream>>);
