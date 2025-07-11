use core::{
  cell::{Cell, SyncUnsafeCell, UnsafeCell}, mem::MaybeUninit, ops::Deref, ptr::{self, NonNull}
};

use crate::alloc::{FreeVtable, OutOfMemory, SliceAllocator, SliceDst, UnsizedMaybeUninit, strategy::Strategy};

use super::calculate_layout_for_dst;

pub struct ArenaAllocator<A: UnsafeCellBuffer> {
  head: Cell<usize>,
  data: A,
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

impl<'s, T: 's, A: UnsafeCellBuffer> Allocator<'s, T> for ArenaAllocator<A> {
  type Error = OutOfMemory;

  async fn reserve_item<S: Strategy>(&'s self) -> Result<S::Handle<'s, MaybeUninit<T>>, OutOfMemory>
  where
    S::Data<'s, MaybeUninit<T>>: Sized,
  {
    // Safety: buffer ptr + head < buffer end ptr, never overflows
    let alignment_offset = unsafe {
      self.data.get().cast::<u8>().add(self.head.get()).align_offset(align_of::<S::Data<'s, MaybeUninit<T>>>())
    };

    let new_head = self
      .head
      .get()
      .checked_add(alignment_offset)
      .ok_or(OutOfMemory)?
      .checked_add(size_of::<S::Data<'s, MaybeUninit<T>>>())
      .ok_or(OutOfMemory)?;

    if new_head > self.len() {
      return Err(OutOfMemory);
    }

    let head = self.head.replace(new_head) + alignment_offset;
    let ptr = ptr::without_provenance_mut(unsafe { self.data.get().byte_add(head).addr() });

    unsafe { S::initialize_data(FreeVtable::new_empty(), ptr) };

    Ok(S::construct_handle_sized(unsafe { NonNull::new_unchecked(ptr) }))
  }
}

impl<'s, T: SliceDst + ?Sized + 's, A: UnsafeCellBuffer> SliceAllocator<'s, T> for ArenaAllocator<A> {
  type Error = OutOfMemory;
  async fn reserve_slice<S: Strategy>(
    &'s self,
    length: usize,
  ) -> Result<S::Handle<'s, UnsizedMaybeUninit<T>>, OutOfMemory>
  where
    S::Data<'s, UnsizedMaybeUninit<T>>: SliceDst,
  {
    let layout = calculate_layout_for_dst::<S::Data<'s, UnsizedMaybeUninit<T>>>(length).map_err(|_| OutOfMemory)?;
    // Safety: buffer ptr + head < buffer end ptr, never overflows
    let alignment_offset = unsafe { self.data.get().cast::<u8>().add(self.head.get()).align_offset(layout.align()) };
    let new_head = self
      .head
      .get()
      .checked_add(alignment_offset)
      .ok_or(OutOfMemory)?
      .checked_add(layout.size())
      .ok_or(OutOfMemory)?;

    if new_head > self.len() {
      return Err(OutOfMemory);
    }

    let head = self.head.replace(new_head);
    let ptr = unsafe { self.data.get().byte_add(head) };
    let ptr = ptr::from_raw_parts_mut(ptr.cast::<u8>(), length);

    unsafe { S::initialize_data(FreeVtable::new_empty(), ptr) };

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
    SliceAllocator, allocator::{ArenaAllocator, OutOfMemory}, strategy::{Unique, UniqueData, UniqueStrategy}
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
    arena.take::<UniqueStrategy>(5u32).await.unwrap();
  }

  #[pollster::test]
  async fn allocate_items() {
    let arena = test_arena::<{ size_of::<UniqueData<u32>>() * 2 }>();
    let _handle = arena.take::<UniqueStrategy>(5u32).await.unwrap();
    let _handle = arena.take::<UniqueStrategy>(5u32).await.unwrap();
    let result = arena.take::<UniqueStrategy>(5u32).await;
    assert_matches!(result, Err(OutOfMemory));
  }

  #[pollster::test]
  async fn allocate_dst() {
    let arena = test_arena::<{ size_of::<UniqueData<u32>>() + 4 }>();
    let _: Unique<u32> = arena.take::<UniqueStrategy>(1).await.unwrap();
    let result: Result<Unique<u32>, _> = arena.take::<UniqueStrategy>(1).await;
    assert_matches!(result, Err(OutOfMemory));
  }

  #[pollster::test]
  async fn allocate_item_oom() {
    let arena = test_arena::<0>();
    let result: Result<Unique<u8>, _> = arena.take::<UniqueStrategy>(1).await;
    assert_matches!(result, Err(OutOfMemory));
  }

  #[pollster::test]
  async fn allocate_dst_oom() {
    let arena = test_arena::<0>();
    let result: Result<Unique<[u8]>, _> = arena.from_zeros::<UniqueStrategy>(1).await;
    assert_matches!(result, Err(OutOfMemory));
  }

  #[pollster::test]
  async fn allocate_dst_overflow() {
    let arena = test_arena::<0>();
    let result: Result<Unique<[u32]>, _> = arena.from_zeros::<UniqueStrategy>(usize::MAX).await;
    assert_matches!(result, Err(OutOfMemory));
  }
}

#[cfg(test)]
pub use tests::test_arena;
