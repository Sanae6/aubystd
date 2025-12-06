use core::hint::unreachable_unchecked;
#[cfg(feature = "libc")]
use core::panic::PanicInfo;

use syscalls::{Sysno, syscall};

use crate::println;

#[cfg(not(any(test, doctest)))]
#[lang = "termination"]
trait Termination: Sized {
  fn value(self) -> isize {
    libc::EXIT_SUCCESS as _
  }
}

#[cfg(not(any(test, doctest)))]
impl Termination for () {}

#[cfg(not(any(test, doctest)))]
#[lang = "start"]
fn start<T: Termination + 'static>(
  main: fn() -> T,
  argc: isize,
  argv: *const *const u8,
  _sigpipe: u8,
) -> isize {
  use crate::platform::active;
  active::rt::handle_args(argc, argv);
  // todo: handle sigpipe
  main().value()
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
  println!("{info}");

  unsafe {
    if let Err(error) = syscall!(Sysno::exit, 1) {
      println!("failed to exit?! {error}")
    }
    unreachable_unchecked()
  }
}

#[cfg(all(feature = "libc"))]
#[lang = "eh_personality"]
fn rust_eh_personality() {
  todo!()
}
