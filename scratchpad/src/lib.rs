use aubystd::alloc::{CStyleAllocator, Malloc};

use core::{
  alloc::Layout, fmt, pin::pin, ptr::NonNull, task::{Context, Poll, Waker}
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
pub struct MallocFmt(Malloc);
impl fmt::Write for MallocFmt {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    let layout = Layout::array::<u8>(s.len() + 1).ok().ok_or(fmt::Error)?;
    let ptr = self.0.alloc(layout).ok().ok_or(fmt::Error)?;
    unsafe {
      ptr.copy_from(NonNull::from_ref(s.as_bytes()).cast(), s.len());
      ptr.byte_add(s.len()).write(0)
    };

    unsafe { libc::printf(ptr.as_ptr().cast()) };
    unsafe { self.0.free(ptr, layout) };

    Ok(())
  }
}

#[macro_export]
macro_rules! println {
  () => {
    unsafe { libc::printf(c"\n".as_ptr()) };
  };
  ($($arg: tt)*) => {
    {
      use ::core::fmt::Write;
      writeln!(&mut $crate::MallocFmt::default(), $($arg)*).expect("failed to print message");
    }
  };
}
