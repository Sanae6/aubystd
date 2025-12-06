use core::fmt;

use syscalls::{Sysno, syscall};

#[doc(hidden)]
pub struct StdoutFormat;

impl StdoutFormat {
  fn write_byte_string(&self, s: &[u8]) {
    unsafe { syscall!(Sysno::write, 1i32, s.as_ptr(), s.len()) }.unwrap();
  }
}

impl fmt::Write for StdoutFormat {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.write_byte_string(s.as_bytes());

    Ok(())
  }
}

#[macro_export]
#[doc(hidden)]
macro_rules! println {
  () => {
    self.write_byte_string(b"\n");
  };
  ($($arg: tt)*) => {
    {
      use ::core::fmt::Write;
      writeln!(&mut $crate::internal::StdoutFormat, $($arg)*).expect("failed to print message");
    }
  };
}
