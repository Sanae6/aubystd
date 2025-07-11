use core::{
  alloc::Layout, mem::MaybeUninit, ptr::{self, NonNull}
};

use crate::alloc::{FreeVtable, SliceAllocator, UnsizedMaybeUninit, strategy::Strategy};

use super::{OutOfMemory, calculate_layout_for_dst};

pub trait CStyleAllocator {
  fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, OutOfMemory>;
  unsafe fn free(&self, ptr: NonNull<u8>, layout: Layout);
}

#[derive(Default)]
pub struct ForeignAllocator<C: CStyleAllocator> {
  allocator: C,
}

impl<C: CStyleAllocator> ForeignAllocator<C> {
  pub fn new(allocator: C) -> Self {
    Self { allocator }
  }

  fn create_free_vtable<'allocator>(&'allocator self) -> FreeVtable<'allocator> {
    FreeVtable::new(Self::free, &raw const *self as *mut Self)
  }

  /// Safety: The context provided to the free function must be a pointer to the allocator.
  unsafe fn free(context: *mut (), allocation: *mut (), layout: Layout) {
    // Safety: See above. The context is never accessed mutably, so we can freely get an immutable reference.
    let this = unsafe { context.cast::<Self>().as_ref().expect("null context was provided") };

    if let Some(ptr) = NonNull::new(allocation) {
      unsafe { this.allocator.free(ptr.cast(), layout) };
    }
  }
}

impl<'s, T: 's, C: CStyleAllocator> Allocator<'s, T> for ForeignAllocator<C> {
  type Error = OutOfMemory;

  async fn reserve_item<S: Strategy>(&'s self) -> Result<S::UninitSizedHandle<'s, T>, OutOfMemory> {
    let layout = Layout::new::<S::SizedData<'s, T>>();

    let data_ptr = self.allocator.alloc(layout)?;
    let data_ptr = data_ptr.cast::<S::SizedData<'s, MaybeUninit<T>>>();

    S::initialize_data_sized(self.create_free_vtable(), data_ptr.as_ptr());

    Ok(S::construct_handle_sized(data_ptr))
  }
}

impl<'s, T: SliceDst + ?Sized + 's, C: CStyleAllocator> SliceAllocator<'s, T> for ForeignAllocator<C> {
  type Error = OutOfMemory;

  async fn reserve_slice<S: Strategy>(&'s self, length: usize) -> Result<S::UninitSliceHandle<'s, T>, OutOfMemory> {
    let layout =
      calculate_layout_for_dst::<S::SliceData<'s, UnsizedMaybeUninit<T>>>(length).map_err(|_| OutOfMemory)?;

    let data_ptr = self.allocator.alloc(layout)?;
    let data_ptr: NonNull<S::SliceData<'s, UnsizedMaybeUninit<T>>> =
      unsafe { NonNull::new_unchecked(ptr::from_raw_parts_mut(data_ptr.as_ptr() as *mut (), length)) };

    S::initialize_data_slice(self.create_free_vtable(), data_ptr.as_ptr());

    Ok(S::construct_handle_slice(data_ptr))
  }
}

#[cfg(feature = "libc")]
#[derive(Default)]
pub struct Malloc;

#[cfg(feature = "libc")]
impl CStyleAllocator for Malloc {
  fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, OutOfMemory> {
    let ptr = unsafe { libc::aligned_alloc(layout.align(), layout.size()) };
    NonNull::new(ptr.cast::<u8>()).ok_or(OutOfMemory)
  }

  unsafe fn free(&self, ptr: NonNull<u8>, _layout: Layout) {
    unsafe { libc::free(ptr.cast().as_ptr()) };
  }
}

#[cfg(any(feature = "alloc", test))]
extern crate alloc as rust_alloc;

#[cfg(any(feature = "alloc", test))]
#[derive(Default)]
pub struct StdAlloc;

#[cfg(any(feature = "alloc", test))]
impl CStyleAllocator for StdAlloc {
  fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, OutOfMemory> {
    let ptr = unsafe { rust_alloc::alloc::alloc(layout) };
    NonNull::new(ptr).ok_or(OutOfMemory)
  }

  unsafe fn free(&self, ptr: NonNull<u8>, layout: Layout) {
    unsafe {
      rust_alloc::alloc::dealloc(ptr.as_ptr(), layout);
    };
  }
}

#[cfg(test)]
pub mod tests {
  use crate::alloc::{
    allocator::{ForeignAllocator, StdAlloc}, strategy::UniqueStrategy
  };

  #[pollster::test]
  async fn allocate_item() {
    let arena = ForeignAllocator::new(StdAlloc);
    arena.take::<UniqueStrategy>(5u32).await.unwrap();
  }

  #[pollster::test]
  async fn allocate_items() {
    let arena = ForeignAllocator::new(StdAlloc);
    let _handle = arena.take::<UniqueStrategy>(5u32).await.unwrap();
  }
}
