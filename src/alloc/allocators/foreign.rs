use core::{
  alloc::Layout, marker::PhantomData, mem::MaybeUninit, ptr::{self, NonNull}
};

use crate::alloc::{Allocator, FreeVtable, UnsizedMaybeUninit, strategy::Strategy};

use super::{AllocateError, OutOfMemory, calculate_layout_for_dst};

pub trait CStyleAllocator {
  fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, OutOfMemory>;
  unsafe fn free(&self, ptr: NonNull<u8>, layout: Layout);
}

#[derive(Default)]
pub struct ForeignAllocator<C: CStyleAllocator, S: Strategy> {
  allocator: C,
  strategy: PhantomData<S>,
}

impl<C: CStyleAllocator, S: Strategy> ForeignAllocator<C, S> {
  pub fn new(allocator: C) -> Self {
    Self {
      allocator,
      strategy: PhantomData,
    }
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

impl<C: CStyleAllocator, S: Strategy> Allocator<S> for ForeignAllocator<C, S> {
  type UnderlyingAllocateError = !;

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
  ) -> Result<S::UninitSizedHandle<'allocator, T>, AllocateError<!>> {
    let layout = Layout::new::<S::SizedData<'allocator, T>>();

    let data_ptr = self.allocator.alloc(layout).map_err(AllocateError::OutOfMemory)?;
    let data_ptr = data_ptr.cast::<S::SizedData<'allocator, MaybeUninit<T>>>();

    S::initialize_data_sized(self.create_free_vtable(), data_ptr.as_ptr());

    Ok(S::construct_handle_sized(data_ptr))
  }

  async fn reserve_dst<'allocator, T: crate::alloc::SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::UninitSliceHandle<'allocator, T>, AllocateError<!>> {
    let layout = calculate_layout_for_dst::<S::SliceData<'allocator, UnsizedMaybeUninit<T>>>(element_count)?;

    let data_ptr = self.allocator.alloc(layout)?;
    let data_ptr: NonNull<S::SliceData<'allocator, UnsizedMaybeUninit<T>>> =
      unsafe { NonNull::new_unchecked(ptr::from_raw_parts_mut(data_ptr.as_ptr() as *mut (), element_count)) };

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
