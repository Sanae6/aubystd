use core::net::SocketAddr;

use crate::{
  io::net::tcp::{TcpClient, TcpSocket}, platform::linux::io::{FileDescriptor, LinuxError}
};
use syscalls::{Sysno, syscall};

use super::LinuxIo;

impl TcpClient for LinuxIo {
  type Error = LinuxError;
  async fn open_connection<'a>(&'a self, address: SocketAddr) -> Result<LinuxSocket, LinuxError> {
    let family = match &address {
      SocketAddr::V4(_) => libc::AF_INET,
      SocketAddr::V6(_) => libc::AF_INET6,
    };

    let _socket = unsafe {
      syscall!(
        Sysno::socket,
        family,
        libc::SOCK_STREAM | libc::SOCK_NONBLOCK,
        libc::IPPROTO_TCP
      )
    }
    .map_err(LinuxError)?;
    
    todo!("register on io worker thread")
  }
}

pub struct LinuxSocket {
  file_descriptor: FileDescriptor,
  local_address: SocketAddr,
  peer_address: SocketAddr,
}

impl TcpSocket for LinuxSocket {
  fn local_address(&self) -> SocketAddr {
    self.local_address
  }

  fn peer_address(&self) -> SocketAddr {
    self.peer_address
  }
}
