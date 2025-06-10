use core::mem::MaybeUninit;

use super::strategy::Strategy;

pub trait ItemAllocator<T, S: Strategy> {
  type AllocateError;

  async fn take<'allocator>(&'allocator self, value: T) -> Result<S::SizedHandle<'allocator, T>, Self::AllocateError>
  where
    T: 'allocator;

  async fn reserve<'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::SizedHandle<'allocator, MaybeUninit<T>>, Self::AllocateError>
  where
    T: 'allocator;
}
