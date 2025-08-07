use core::{alloc::Layout, fmt, ptr::NonNull};

use syscalls::{syscall, Sysno};

use crate::alloc::{CStyleAllocator, Malloc, MemoryMapped};

#[derive(Default)]
#[doc(hidden)]
pub struct StdoutFormat;
impl fmt::Write for StdoutFormat {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    unsafe {
      syscall!(
        Sysno::write,
        1i32,
        s.as_ptr(),
        s.len()
      )
    }.unwrap();

    Ok(())
  }
}

#[macro_export]
#[doc(hidden)]
macro_rules! println {
  () => {
    unsafe { libc::printf(c"\n".as_ptr()) };
  };
  ($($arg: tt)*) => {
    {
      use ::core::fmt::Write;
      writeln!(&mut $crate::internal::StdoutFormat::default(), $($arg)*).expect("failed to print message");
    }
  };
}
