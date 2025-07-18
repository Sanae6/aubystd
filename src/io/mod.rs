use core::error::Error;

// pub mod fs;
pub mod net;
mod platform;
pub mod stdio;

#[cfg(target_os = "linux")]
pub use platform::TargetIo;

pub trait Io {}

pub trait StreamWrite {
  type Error: Error;

  /// Writes `data` to the stream. This operation can end without writing all of the data passed.
  /// Returns the amount of bytes that have been written, and the remaining slice of `data`.
  async fn write<'a>(&mut self, data: &'a [u8]) -> Result<(usize, &'a [u8]), Self::Error>;

  /// Writes all of `data` to the stream.
  async fn write_all<'a>(&mut self, mut data: &'a [u8]) -> Result<(), Self::Error> {
    loop {
      let (_, remaining) = self.write(data).await?;

      data = remaining;
    }
  }
}

pub trait StreamRead {
  type Error: Error;

  async fn read<'a, 'b>(&'b mut self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error>;
}

pub trait StreamPeek {
  type Error: Error;

  async fn peek<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error>;
}

pub trait StreamSplit {
  type Error: Error;
  async fn split(self) -> (impl StreamRead, impl StreamWrite);
}

pub trait DatagramReader {
  type Error: Error;

  async fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;
}

pub trait DatagramWriter {
  type Error: Error;

  async fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
}
