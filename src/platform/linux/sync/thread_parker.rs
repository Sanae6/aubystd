use core::sync::atomic::{AtomicU32, Ordering};

use libc::FUTEX_WAIT;
use syscalls::{Errno, Sysno, syscall};

#[repr(u32)]
enum LockState {
  Running,
  Waiting,
}

pub struct LinuxThreadParker {
  value: AtomicU32,
}

impl LinuxThreadParker {
  pub const fn new() -> Self {
    Self {
      value: AtomicU32::new(0),
    }
  }

  pub fn sleep(&self) {
    self.value.store(LockState::Waiting as _, Ordering::Relaxed);
    loop {
      let result = unsafe {
        syscall!(
          Sysno::futex,
          &raw const self.value,
          FUTEX_WAIT,
          LockState::Waiting,
          0,
          0,
          0
        )
      };

      if let Err(error) = result
        && error == Errno::EAGAIN
      {
        return;
      }

      if self.value.load(Ordering::Acquire) == LockState::Running as _ {
        return;
      }
    }
  }

  pub fn wake(&self) {
    if let Ok(_) = self.value.compare_exchange(
      LockState::Waiting as _,
      LockState::Running as _,
      Ordering::Release,
      Ordering::Relaxed,
    ) {
      unsafe {
        syscall!(
          Sysno::futex,
          &raw const self.value,
          libc::FUTEX_WAKE,
          LockState::Waiting,
          0,
          0,
          0
        )
        .unwrap();
      }
    }
  }
}
