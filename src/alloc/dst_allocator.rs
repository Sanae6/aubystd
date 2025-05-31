use core::mem::MaybeUninit;

use zerocopy::FromZeros;

use super::Strategy;

pub trait DstAllocator<S: Strategy> {
  type AllocateError;

  async fn take_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>;

  async fn take_array<'allocator, T: 'allocator, const N: usize>(
    &'allocator self,
    value: [T; N],
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>;

  async fn take_from_iter<'allocator, T: 'allocator>(
    &'allocator self,
    iterator: impl ExactSizeIterator<Item = T>,
  ) -> Result<S::Handle<'allocator, [T]>, Self::AllocateError>;

  async fn take_from_zeros<'allocator, T: FromZeros + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, [T]>, Self::AllocateError>;


  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, MaybeUninit<T>>, Self::AllocateError>;

  async fn reserve_array<'allocator, T: 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, [MaybeUninit<T>]>, Self::AllocateError>;
}
