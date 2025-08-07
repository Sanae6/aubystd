use core::{alloc::Layout, mem::MaybeUninit, ptr};

use crate::alloc::{
  FreeVtable, LayoutAllocator, SliceAllocator, UnsizedMaybeUninit, strategy::Strategy,
};

use super::{OutOfMemory, calculate_layout_for_dst};

#[cfg(feature = "libc")]
mod malloc;
/// todo: support windows
#[cfg(unix)]
pub mod mmap;
#[cfg(any(feature = "alloc", test))]
mod std_alloc;

#[cfg(feature = "libc")]
pub use malloc::Malloc;
#[cfg(unix)]
#[doc(inline)]
pub use mmap::MemoryMapped;
#[cfg(any(feature = "alloc", test))]
pub use std_alloc::StdAlloc;

/// A memory allocator adapter for C-style allocators (malloc and free).
///
/// Safety:
/// Allocated memory blocks must be valid, and remain until they are freed.
///
pub unsafe trait CStyleAllocator {
  fn alloc(&self, layout: Layout) -> Result<ptr::NonNull<u8>, OutOfMemory>;
  /// Safety:
  /// - `ptr` must point to a valid memory block allocated by this allocator
  /// - `layout` must be the layout of the memory block
  unsafe fn free(&self, ptr: ptr::NonNull<u8>, layout: Layout);
}

#[derive(Default)]
pub struct ForeignAllocator<C: CStyleAllocator> {
  allocator: C,
}

impl<C: CStyleAllocator> ForeignAllocator<C> {
  pub const fn new(allocator: C) -> Self {
    Self { allocator }
  }

  fn create_free_vtable<'a>(&'a self) -> FreeVtable<'a> {
    FreeVtable::new(Self::free, &raw const self.allocator as *mut Self)
  }

  /// Safety contract:
  /// - the context provided must be a pointer to the allocator
  /// - the allocation provided must point to an allocated block of memory
  unsafe fn free(context: *const (), allocation: *const (), layout: Layout) {
    // Safety: the context is never accessed mutably, so we can freely get an immutable reference.
    let c_allocator = unsafe {
      context
        .cast::<C>()
        .as_ref()
        .expect("null context was provided")
    };

    if let Some(ptr) = ptr::NonNull::new(allocation.cast_mut()) {
      // Safety: the allocation is a valid pointer
      unsafe { c_allocator.free(ptr.cast(), layout) };
    }
  }
}

impl<'s, T: 's, C: CStyleAllocator> Allocator<'s, T> for ForeignAllocator<C> {
  type Error = OutOfMemory;

  async fn reserve_item<S: Strategy>(
    &'s self,
  ) -> Result<S::UninitHandle<'s, MaybeUninit<T>>, OutOfMemory>
  where
    S::Data<'s, MaybeUninit<T>>: Sized,
  {
    let layout = Layout::new::<S::Data<'s, MaybeUninit<T>>>();

    let data_ptr = self.allocator.alloc(layout)?;
    let data_ptr = data_ptr.cast::<S::Data<'s, MaybeUninit<T>>>();

    // Safety: alloc only returns valid, well aligned pointers for the provided layout
    unsafe { S::initialize_data(self.create_free_vtable(), data_ptr.as_ptr()) };

    Ok(S::construct_handle(data_ptr))
  }
}

impl<'s, T: SliceDst + ?Sized + 's, C: CStyleAllocator> SliceAllocator<'s, T>
  for ForeignAllocator<C>
{
  type Error = OutOfMemory;

  async fn reserve_slice<S: Strategy>(
    &'s self,
    length: usize,
  ) -> Result<S::UninitHandle<'s, UnsizedMaybeUninit<T>>, OutOfMemory>
  where
    S::Data<'s, UnsizedMaybeUninit<T>>: SliceDst,
  {
    let layout = calculate_layout_for_dst::<S::Data<'s, UnsizedMaybeUninit<T>>>(length)
      .map_err(|_| OutOfMemory)?;

    let data_ptr = self.allocator.alloc(layout)?;
    let data_ptr: ptr::NonNull<S::Data<'s, UnsizedMaybeUninit<T>>> =
      ptr::NonNull::from_raw_parts(data_ptr, length);

    // Safety: alloc only returns valid, well aligned pointers for the provided layout
    unsafe { S::initialize_data(self.create_free_vtable(), data_ptr.as_ptr()) };

    Ok(S::construct_handle(data_ptr))
  }
}

impl<C: CStyleAllocator> LayoutAllocator for ForeignAllocator<C> {
  type Error = OutOfMemory;

  async fn reserve_layout<'s, S: Strategy>(
    &'s self,
    layout: Layout,
  ) -> Result<S::Handle<'s, [MaybeUninit<u8>]>, Self::Error>
  where
    S::Data<'s, ()>: Sized,
    S::Data<'s, [MaybeUninit<u8>]>: ptr::Pointee<Metadata = usize>,
  {
    let new_layout = Layout::new::<S::Data<'s, ()>>()
      .extend(layout)
      .map_err(|_| OutOfMemory)?
      .0
      .pad_to_align();
    let data_ptr = self.allocator.alloc(new_layout)?;
    let data_ptr: ptr::NonNull<S::Data<'s, UnsizedMaybeUninit<[MaybeUninit<u8>]>>> =
      ptr::NonNull::from_raw_parts(data_ptr, new_layout.size());

    // Safety: alloc only returns valid, well aligned pointers for the provided layout
    unsafe { S::initialize_data(self.create_free_vtable(), data_ptr.as_ptr()) };

    let handle: S::UninitHandle<'s, UnsizedMaybeUninit<[MaybeUninit<u8>]>> =
      S::construct_handle(data_ptr);
    Ok(unsafe { S::UninitHandle::assume_init(handle) })
  }
}

#[cfg(test)]
#[cfg(feature = "libc")]
pub mod tests {
  use crate::alloc::{
    allocator::{ForeignAllocator, StdAlloc},
    strategy::UniqueStrategy,
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
