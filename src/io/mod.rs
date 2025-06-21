pub mod fs;

pub trait StreamWriter {
  type WriteError;
  async fn write(&mut self, buf: &mut &[u8]) -> Result<(), Self::WriteError>;
}

pub trait StreamReader {
  type ReadError;
  async fn read(&mut self, buf: &mut &mut [u8]) -> Result<(), Self::ReadError>;
}

pub trait DatagramReader {
  type ReadError;
  async fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::ReadError>;
}

pub trait DatagramWriter {
  type WriteError;
  async fn write(&mut self, buf: &[u8]) -> Result<(), Self::WriteError>;
}
