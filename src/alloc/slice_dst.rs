use core::ptr::Pointee;

pub use aubystd_macros::SliceDst;

pub trait SliceDst: Pointee<Metadata = usize> {
  type Header;
  type Element;

  fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element];
}

impl<T> SliceDst for [T] {
  type Header = ();
  type Element = T;

  fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element] {
    ptr
  }
}
