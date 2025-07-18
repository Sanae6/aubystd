use crate::io::{Io, StreamRead, StreamWrite};

pub trait Stdin: Io {
  fn stdin_stream(&self) -> impl StreamRead;
}

pub trait Stdout: Io {
  fn stdout_stream(&self) -> impl StreamWrite;
}

pub trait Stderr: Io {
  fn stderr_stream(&self) -> impl StreamWrite;
}
