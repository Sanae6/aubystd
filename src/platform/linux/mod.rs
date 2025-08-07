use core::{
  marker::PhantomData,
  ptr::{self, Pointee},
};

pub mod io;
pub mod rt;
mod sync;
pub mod thread;

// these are unix types, and exposed under active on unix platforms
#[cfg(unix)]
pub type MaybeFileDescriptor = i32;
#[cfg(unix)]
pub type FileDescriptor = u32;

type ProcessId = libc::pid_t;

#[derive(Clone, Copy)]
struct U64Ptr<T: ?Sized>(u64, PhantomData<*mut T>);

impl<T: ?Sized> U64Ptr<T> {
  pub fn new(ptr: *mut T) -> Self {
    Self(ptr.expose_provenance() as u64, PhantomData)
  }

  pub fn null() -> Self {
    Self(0, PhantomData)
  }

  pub fn value_as_usize(self) -> usize {
    self.0 as usize
  }
}

impl<T: ?Sized + Pointee<Metadata = ()>> U64Ptr<T> {
  pub fn value(self) -> *mut T {
    ptr::null_mut::<T>().with_addr(self.value_as_usize())
  }
}
