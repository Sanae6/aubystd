use crate::alloc::SliceDst;
use core::{mem::MaybeUninit, ptr};

#[derive(SliceDst)]
#[repr(C)]
pub struct UnsizedMaybeUninit<T: SliceDst + ?Sized> {
  pub header: MaybeUninit<T::Header>,
  pub slice: [MaybeUninit<T::Element>],
}

impl<T: SliceDst + ?Sized> UnsizedMaybeUninit<T> {
  fn as_mut_ptr(&mut self) -> *mut T {
    let (ptr, size) = ptr::from_mut(self).to_raw_parts();

    ptr::from_raw_parts_mut(ptr, size)
  }
}
