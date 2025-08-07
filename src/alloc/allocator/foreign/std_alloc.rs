use crate::alloc::{CStyleAllocator, OutOfMemory};
use core::{alloc::Layout, ptr};

extern crate alloc as rust_alloc;

#[derive(Default)]
pub struct StdAlloc;

unsafe impl CStyleAllocator for StdAlloc {
  fn alloc(&self, layout: Layout) -> Result<ptr::NonNull<u8>, OutOfMemory> {
    let ptr = unsafe { rust_alloc::alloc::alloc(layout) };
    ptr::NonNull::new(ptr).ok_or(OutOfMemory)
  }

  unsafe fn free(&self, ptr: ptr::NonNull<u8>, layout: Layout) {
    unsafe {
      rust_alloc::alloc::dealloc(ptr.as_ptr(), layout);
    };
  }
}
