use syscalls::Errno;
use thiserror::Error;

use crate::io::Io;

mod tcp;

pub struct LinuxIo {
  a: bool,
}

impl LinuxIo {
  fn instance() -> &'static Self {
  }
}

impl Io for LinuxIo {}

// todo: OsError for aggregating platforms
// todo: enum
#[derive(Error, Debug)]
#[non_exhaustive]
#[error("Unhandled error: {0}")]
pub struct LinuxError(Errno);
