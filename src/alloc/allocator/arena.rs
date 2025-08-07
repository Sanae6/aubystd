use core::{
  alloc::Layout,
  cell::{Cell, SyncUnsafeCell, UnsafeCell},
  mem::MaybeUninit,
  ops::Deref,
  ptr,
};

use crate::alloc::{
  FreeVtable, LayoutAllocator, OutOfMemory, SliceAllocator, SliceDst, UnsizedMaybeUninit,
  strategy::Strategy,
};

use super::calculate_layout_for_dst;

pub struct ArenaAllocator<A: UnsafeCellBuffer> {
  head: Cell<usize>,
  data: A,
}

impl<A: UnsafeCellBuffer> ArenaAllocator<A> {
  pub fn new(data: A) -> Self {
    Self {
      head: 0.into(),
      data,
    }
  }

  pub fn len(&self) -> usize {
    self.data.get().len()
  }

  pub fn remaining(&self) -> usize {
    self.len() - self.head.get()
  }

  fn fetch_head_ptr<'s, S: Strategy>(
    &'s self,
    layout: Layout,
  ) -> Result<ptr::NonNull<()>, OutOfMemory> {
    // Safety: buffer ptr + head < buffer end ptr, never overflows
    let alignment_offset = unsafe {
      self
        .data
        .get()
        .cast::<u8>()
        .add(self.head.get())
        .align_offset(layout.align())
    };

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

    let head = self.head.replace(new_head) + alignment_offset;

    // Safety: data ptr can never be null
    Ok(unsafe { ptr::NonNull::new_unchecked(self.data.get().byte_add(head).cast()) })
  }
}

impl<'s, T: 's, A: UnsafeCellBuffer> Allocator<'s, T> for ArenaAllocator<A> {
  type Error = OutOfMemory;

  async fn reserve_item<S: Strategy>(
    &'s self,
  ) -> Result<S::UninitHandle<'s, MaybeUninit<T>>, OutOfMemory>
  where
    S::Data<'s, MaybeUninit<T>>: Sized,
  {
    let ptr = self
      .fetch_head_ptr::<S>(Layout::new::<S::Data<'s, MaybeUninit<T>>>())?
      .cast();

    unsafe { S::initialize_data(FreeVtable::new_empty(), ptr.as_ptr()) };

    Ok(S::construct_handle(ptr))
  }
}

impl<'s, T: SliceDst + ?Sized + 's, A: UnsafeCellBuffer> SliceAllocator<'s, T>
  for ArenaAllocator<A>
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

    let ptr = self.fetch_head_ptr::<S>(layout)?;
    let ptr: ptr::NonNull<S::Data<'s, UnsizedMaybeUninit<T>>> =
      ptr::NonNull::from_raw_parts(ptr, length);

    unsafe { S::initialize_data(FreeVtable::new_empty(), ptr.as_ptr()) };

    Ok(S::construct_handle(ptr))
  }
}

impl<A: UnsafeCellBuffer> LayoutAllocator for ArenaAllocator<A> {
  type Error = OutOfMemory;

  async fn reserve_layout<'s, S: Strategy>(
    &'s self,
    layout: core::alloc::Layout,
  ) -> Result<S::Handle<'s, [MaybeUninit<u8>]>, Self::Error>
  where
    S::Data<'s, ()>: Sized,
  {
    let new_layout = Layout::new::<S::Data<'s, ()>>()
      .extend(layout)
      .map_err(|_| OutOfMemory)?
      .0
      .pad_to_align();
    let ptr = self.fetch_head_ptr::<S>(new_layout)?;
    let ptr: ptr::NonNull<S::Data<'s, UnsizedMaybeUninit<[MaybeUninit<u8>]>>> =
      ptr::NonNull::from_raw_parts(ptr, new_layout.size());

    unsafe { S::initialize_data(FreeVtable::new_empty(), ptr.as_ptr()) };

    let handle: S::UninitHandle<'s, _> = S::construct_handle(ptr);
    Ok(unsafe { S::UninitHandle::assume_init(handle) })
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
#[cfg(feature = "libc")]
mod tests {
  use core::{cell::UnsafeCell, mem::MaybeUninit};

  use crate::alloc::{
    ForeignAllocator, Malloc, SliceAllocator, UnsafeCellBuffer,
    allocator::{ArenaAllocator, OutOfMemory},
    strategy::{Strategy, Unique, UniqueStrategy},
  };

  pub async fn test_arena<S: Strategy, T: 'static>(
    count: usize,
  ) -> ArenaAllocator<impl UnsafeCellBuffer>
  where
    S::Data<'static, T>: Sized,
  {
    extern crate alloc;
    use alloc::boxed::Box;
    let allocator = Box::leak(Box::new(ForeignAllocator::new(Malloc)));

    let buffer: Unique<UnsafeCell<[MaybeUninit<u8>]>> = allocator
      .from_zeros::<UniqueStrategy>(size_of::<S::Data<'static, T>>() * count)
      .await
      .unwrap();

    ArenaAllocator::new(buffer)
  }

  #[macro_export]
  macro_rules! test_arena {
    ($type: tt, $count: literal) => {{ test_arena::<$type, u32>($count) }};
  }

  // #[pollster::test]
  async fn allocate_item() {
    let arena = test_arena!(UniqueStrategy, 2).await;
    arena.take::<UniqueStrategy>(5u32).await.unwrap();
  }

  #[pollster::test]
  async fn allocate_items() {
    let arena = test_arena!(UniqueStrategy, 1).await;
    let _handle = arena.take::<UniqueStrategy>(5u32).await.unwrap();
    // let _handle = arena.take::<UniqueStrategy>(5u32).await.unwrap();
    let result = arena.take::<UniqueStrategy>(5u32).await;
    assert!(matches!(result, Err(OutOfMemory)));
  }

  #[pollster::test]
  async fn allocate_dst() {
    let arena = test_arena!(UniqueStrategy, 1).await;
    let _: Unique<u32> = arena.take::<UniqueStrategy>(1).await.unwrap();
    let result: Result<Unique<u32>, _> = arena.take::<UniqueStrategy>(1).await;
    assert!(matches!(result, Err(OutOfMemory)));
  }

  #[pollster::test]
  async fn allocate_item_oom() {
    let arena = test_arena!(UniqueStrategy, 0).await;
    let result: Result<Unique<u8>, _> = arena.take::<UniqueStrategy>(1).await;
    assert!(matches!(result, Err(OutOfMemory)));
  }

  #[pollster::test]
  async fn allocate_dst_oom() {
    let arena = test_arena!(UniqueStrategy, 0).await;
    let result: Result<Unique<[u8]>, _> = arena.from_zeros::<UniqueStrategy>(1).await;
    assert!(matches!(result, Err(OutOfMemory)));
  }

  #[pollster::test]
  async fn allocate_dst_overflow() {
    let arena = test_arena!(UniqueStrategy, 0).await;
    let result: Result<Unique<[u32]>, _> = arena.from_zeros::<UniqueStrategy>(usize::MAX).await;
    assert!(matches!(result, Err(OutOfMemory)));
  }
}

#[cfg(test)]
#[cfg(feature = "libc")]
pub use tests::test_arena;
