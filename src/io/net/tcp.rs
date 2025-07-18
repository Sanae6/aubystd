use core::{error::Error, net::SocketAddr};

use crate::io::{Io, StreamRead, StreamWrite};

pub trait TcpClient: Io {
  type Error: Error;

  // todo: cancellation???
  async fn open_connection<'a>(&'a self, address: SocketAddr) -> Result<impl TcpSocket + 'a, Self::Error>;
}

pub trait TcpServer: Io {
  type Error: Error;

  async fn listen(&self, address: SocketAddr) -> Result<impl TcpListener, Self::Error>;
}

pub trait TcpListener {
  type Error: Error;

  async fn accept(&mut self)-> Result<impl TcpSocket + StreamRead + StreamWrite, Self::Error>;
}

pub trait TcpSocket {
  fn local_address(&self) -> SocketAddr;
  fn peer_address(&self) -> SocketAddr;
}

pub trait TcpSocketNagle: TcpSocket {
  type Error: Error;

  fn set_nagle_algorithm(&self, value: bool) -> Result<(), <Self as TcpSocketNagle>::Error>;
}
