use syscalls::Errno;
use thiserror::Error;

use crate::io::Io;

mod tcp;

pub struct LinuxIo;

impl LinuxIo {
  pub fn new() -> Self {
    Self
  }
}

impl Io for LinuxIo {}

// todo: OsError for aggregating platforms
// todo: enum
#[derive(Error, Debug)]
#[non_exhaustive]
#[error("Unhandled error: {0}")]
pub struct LinuxError(Errno);

type MaybeFileDescriptor = i32;
type FileDescriptor = u32;
