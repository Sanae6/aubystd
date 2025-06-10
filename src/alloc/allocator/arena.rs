use core::{
  cell::{Cell, SyncUnsafeCell, UnsafeCell}, mem::MaybeUninit, ops::Deref, ptr::{self, NonNull}
};

use crate::alloc::{FreeVtable, SliceDst, allocator::OutOfMemory, strategy::Strategy};

use super::{AllocateError, OverflowedLayoutCalculation, calculate_layout_for_dst};

pub struct ArenaAllocator<A: UnsafeCellBuffer> {
  head: Cell<usize>,
  data: A
}

impl<A: UnsafeCellBuffer> ArenaAllocator<A> {
  pub fn new(data: A) -> Self {
    Self { head: 0.into(), data }
  }

  pub fn len(&self) -> usize {
    self.data.get().len()
  }

  pub fn remaining(&self) -> usize {
    self.len() - self.head.get()
  }
}

impl<A: UnsafeCellBuffer> Allocator for ArenaAllocator<A> {
  type UnderlyingAllocateError = !;

  async fn reserve_item<'allocator, S: Strategy, T: 'allocator>(
    &'allocator self,
    _: S,
  ) -> Result<S::UninitSizedHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>> {
    let new_head =
      self.head.get().checked_add(size_of::<S::SizedData<'allocator, T>>()).ok_or(OverflowedLayoutCalculation)?;

    if new_head > self.len() {
      return Err(OutOfMemory.into());
    }

    let head = self.head.replace(new_head);
    let ptr = unsafe { self.data.get().byte_add(head).cast() };

    S::initialize_data_sized(FreeVtable::new_empty(), ptr);

    Ok(S::construct_handle_sized(unsafe { NonNull::new_unchecked(ptr) }))
  }

  async fn reserve_dst<'allocator, S: Strategy, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    _: S,
    element_count: usize,
  ) -> Result<S::UninitSliceHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>> {
    let layout = calculate_layout_for_dst::<S::SliceData<'allocator, T>>(element_count)?;
    let new_head = self.head.get().checked_add(layout.size()).ok_or(OverflowedLayoutCalculation)?;
    if new_head > self.len() {
      return Err(OutOfMemory.into());
    }

    let head = self.head.replace(new_head);
    let ptr = unsafe { self.data.get().byte_add(head) };
    let ptr = ptr::from_raw_parts_mut(ptr.cast::<u8>(), element_count);

    S::initialize_data_slice(FreeVtable::new_empty(), ptr);

    Ok(S::construct_handle_slice(unsafe { NonNull::new_unchecked(ptr) }))
  }
}

pub trait UnsafeCellBuffer {
  fn get(&self) -> *mut [MaybeUninit<u8>];
}

impl UnsafeCellBuffer for UnsafeCell<[MaybeUninit<u8>]> {
  fn get(&self) -> *mut [MaybeUninit<u8>] {
    UnsafeCell::get(self)
  }
}

impl UnsafeCellBuffer for SyncUnsafeCell<[MaybeUninit<u8>]> {
  fn get(&self) -> *mut [MaybeUninit<u8>] {
    SyncUnsafeCell::get(self)
  }
}

impl<T: UnsafeCellBuffer + ?Sized, D: Deref<Target = T>> UnsafeCellBuffer for D {
  fn get(&self) -> *mut [MaybeUninit<u8>] {
    T::get(self.deref())
  }
}

#[cfg(test)]
mod tests {
  use core::{cell::UnsafeCell, mem::MaybeUninit};
  use std::assert_matches::assert_matches;

  use crate::alloc::{
    allocator::{AllocateError, ArenaAllocator}, strategy::{UNIQUE, Unique, UniqueData}
  };

  pub fn test_arena<const SIZE: usize>() -> ArenaAllocator<Box<UnsafeCell<[MaybeUninit<u8>]>>> {
    extern crate alloc;
    use alloc::boxed::Box;
    let buffer: Box<UnsafeCell<[MaybeUninit<u8>]>> = Box::new(UnsafeCell::new([MaybeUninit::uninit(); SIZE]));
    ArenaAllocator::new(buffer)
  }

  #[pollster::test]
  async fn allocate_item() {
    let arena = test_arena::<{ size_of::<UniqueData<u32>>() + 4 }>();
    arena.take_item(UNIQUE, 5u32).await.unwrap();
  }

  #[pollster::test]
  async fn allocate_items() {
    let arena = test_arena::<{ size_of::<UniqueData<u32>>() * 2 }>();
    let _handle = arena.take_item(UNIQUE, 5u32).await.unwrap();
    let _handle = arena.take_item(UNIQUE, 5u32).await.unwrap();
    let result = arena.take_item(UNIQUE, 5u32).await;
    assert_matches!(result, Err(AllocateError::OutOfMemory(_)));
  }

  #[pollster::test]
  async fn allocate_dst() {
    let arena = test_arena::<{ size_of::<UniqueData<u32>>() + 4 }>();
    let _: Unique<[u32]> = arena.take_from_zeros(UNIQUE, 1).await.unwrap();
    let result: Result<Unique<[u32]>, _> = arena.take_from_zeros(UNIQUE, 1).await;
    assert_matches!(result, Err(AllocateError::OutOfMemory(_)));
  }

  #[pollster::test]
  async fn allocate_item_oom() {
    let arena = test_arena::<0>();
    let result: Result<Unique<MaybeUninit<[u8; 1]>>, _> = arena.reserve_item(UNIQUE).await;
    assert_matches!(result, Err(AllocateError::OutOfMemory(_)));
  }

  #[pollster::test]
  async fn allocate_dst_oom() {
    let arena = test_arena::<0>();
    let result: Result<Unique<[u32]>, _> = arena.take_from_zeros(UNIQUE, 42).await;
    assert_matches!(result, Err(AllocateError::OutOfMemory(_)));
  }

  #[pollster::test]
  async fn allocate_dst_overflow() {
    let arena = test_arena::<0>();
    let result: Result<Unique<[u32]>, _> = arena.take_from_zeros(UNIQUE, usize::MAX).await;
    assert_matches!(result, Err(AllocateError::OverflowedLayoutCalculation(_)));
  }
}

#[cfg(test)]
pub use tests::test_arena;
