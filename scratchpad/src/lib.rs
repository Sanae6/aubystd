use aubystd::alloc::{CStyleAllocator, Malloc};
use syscalls::{syscall, Sysno};

use core::{
  alloc::Layout,
  fmt,
  pin::pin,
  ptr::NonNull,
  task::{Context, Poll, Waker},
};

/// replacement for pollster for non-blocking futures
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
  let mut future = pin!(future);

  let mut cx = Context::from_waker(Waker::noop());
  loop {
    match future.as_mut().poll(&mut cx) {
      Poll::Ready(value) => return value,
      Poll::Pending => continue,
    }
  }
}

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
      writeln!(&mut $crate::StdoutFormat::default(), $($arg)*).expect("failed to print message");
    }
  };
}
