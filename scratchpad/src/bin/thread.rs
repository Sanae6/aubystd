use core::sync::atomic::{AtomicBool, Ordering};

use aubystd::{platform::linux::thread::LinuxThreading, thread::Threading};
use libc::sched_yield;
use scratchpad::{block_on, println};

static DONE: AtomicBool = AtomicBool::new(false);
fn main() {
  println!("spawning child");
  let handle = block_on(LinuxThreading.spawn(0x4000, |parker| {
    println!("thread started");
    parker.park();
    println!("thread unparked");
    parker.park();
    println!("thread unbarked");
    DONE.store(true, Ordering::Relaxed)
  }));

  println!("main running");
  handle.unpark().unwrap();
  println!("main yielded");
  loop {
    unsafe { sched_yield() };
    if DONE.load(Ordering::Relaxed) {
      break;
    }
    handle.unpark().unwrap();
  }

  println!("main is back");
}
