use core::{
  mem::MaybeUninit, ptr::Pointee
};

use zerocopy::FromZeros;

use super::strategy::Strategy;

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
    iterator: impl ExactSizeIterator<Item = T::Element>,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>;

  async fn take_from_zeros<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>
  where
    T::Element: FromZeros; // T::Header is always FromZeros.

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, MaybeUninit<T>>, Self::AllocateError>;

  async fn reserve_dst<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError> {
    let _ = element_count;
    todo!("MaybeUninit<T> for unsized types")
  }
}

pub trait SliceDst: Pointee<Metadata = usize> {
  type Header: FromZeros;
  type Element;

  fn addr_of_elements(ptr: *mut Self) -> *mut [Self::Element];
}

impl<T> SliceDst for [T] {
  type Header = ();
  type Element = T;

  fn addr_of_elements(ptr: *mut Self) -> *mut [Self::Element] {
    ptr
  }
}
