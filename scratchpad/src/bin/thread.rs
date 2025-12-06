use core::sync::atomic::{AtomicBool, Ordering};

use aubystd::{platform::linux::thread::LinuxThreading, thread::Threading};
use scratchpad::block_on;
use syscalls::{syscall, Sysno};

static DONE: AtomicBool = AtomicBool::new(false);
fn main() {
  println!("spawning child");
  let handle = block_on(LinuxThreading.spawn(0x4000, |parker| {
    println!("thread started");
    parker.park();
    DONE.store(true, Ordering::Relaxed);
    println!("thread unparked");
    parker.park();
    DONE.store(true, Ordering::Relaxed);
    println!("thread unbarked");
  }));

  println!("main running");
  handle.unpark().unwrap();
  println!("main yielded");
  let wait_park = ||
  loop {
    unsafe { syscall!(Sysno::sched_yield).unwrap(); }
    if DONE.load(Ordering::Relaxed) {
        DONE.store(false, Ordering::Relaxed);
      break;
    }
    handle.unpark().unwrap();
    println!("trying to unpark");
  };
  wait_park();
  wait_park();
  println!("main is back");
  loop {
    unsafe { syscall!(Sysno::sched_yield).unwrap(); }
  }
  panic!("test")
}
