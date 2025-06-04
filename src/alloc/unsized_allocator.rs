use core::{iter::TrustedLen, mem::MaybeUninit};

use zerocopy::FromZeros;

use super::{SliceDst, Strategy, UnsizedMaybeUninit};

pub trait Allocator<S: Strategy> {
  type AllocateError;

  async fn take_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>;

  async fn take_array<'allocator, T: 'allocator, const N: usize>(
    &'allocator self,
    value: [T; N],
  ) -> Result<S::Handle<'allocator, [T; N]>, Self::AllocateError>;

  async fn take_from_iter<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    iterator: impl TrustedLen<Item = T::Element>,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>;

  async fn take_from_zeros<'allocator, T: SliceDst + FromZeros + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>
  where
    T::Element: FromZeros;

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
  ) -> Result<S::Handle<'allocator, MaybeUninit<T>>, Self::AllocateError>;

  async fn reserve_dst<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, UnsizedMaybeUninit<T>>, Self::AllocateError>;
}
