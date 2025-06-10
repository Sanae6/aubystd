use core::{cell::UnsafeCell, ptr::Pointee};

pub use aubystd_macros::SliceDst;

// todo: describe unsafe contract (repr assertion, used for allocations so must match type)
pub unsafe trait SliceDst: Pointee<Metadata = usize> {
  type Header;
  type Element;

  fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element];
}

unsafe impl<T> SliceDst for [T] {
  type Header = ();
  type Element = T;

  fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element] {
    ptr
  }
}

unsafe impl SliceDst for str {
  type Header = ();
  type Element = u8;

  fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element] {
    ptr as *mut _
  }
}

unsafe impl<T: SliceDst + ?Sized> SliceDst for UnsafeCell<T> {
  type Header = T::Header;
  type Element = T::Element;

  fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element] {
    T::addr_of_slice(UnsafeCell::raw_get(ptr as *const _))
  }
}
