#[cfg(feature = "libc")]
use core::panic::PanicInfo;

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

#[cfg(all(feature = "libc"))]
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
  use core::fmt::Write;

  use crate::alloc::Malloc;

  #[derive(Default)]
  pub struct MallocFmt(Malloc);
  impl Write for MallocFmt {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
      use core::{alloc::Layout, fmt, ptr::NonNull};

      use crate::alloc::CStyleAllocator;

      let layout = Layout::array::<u8>(s.len() + 1).ok().ok_or(fmt::Error)?;
      let ptr = self.0.alloc(layout).ok().ok_or(fmt::Error)?;
      unsafe {
        ptr.copy_from(NonNull::from_ref(s.as_bytes()).cast(), s.len());
        ptr.byte_add(s.len()).write(0)
      };

      unsafe { libc::write(libc::STDERR_FILENO, ptr.as_ptr().cast(), s.len()) };

      Ok(())
    }
  }

  if let Err(_) = writeln!(&mut MallocFmt::default(), "{info}") {
    unsafe { libc::printf(c"error while formatting\n".as_ptr()) };
  }

  unsafe {
    // todo: io::exit
    libc::exit(1)
  }
}

#[cfg(all(feature = "libc"))]
#[lang = "eh_personality"]
fn rust_eh_personality() {
  todo!()
}
