use crate::alloc::{CStyleAllocator, OutOfMemory};
use core::{alloc::Layout, ptr};

#[derive(Default)]
pub struct Malloc;

unsafe impl CStyleAllocator for Malloc {
  fn alloc(&self, layout: Layout) -> Result<ptr::NonNull<u8>, OutOfMemory> {
    let ptr = unsafe { libc::aligned_alloc(layout.align(), layout.size()) };
    ptr::NonNull::new(ptr.cast::<u8>()).ok_or(OutOfMemory)
  }

  unsafe fn free(&self, ptr: ptr::NonNull<u8>, _layout: Layout) {
    unsafe { libc::free(ptr.cast().as_ptr()) };
  }
}
