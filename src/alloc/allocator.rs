use core::{error::Error, iter::TrustedLen, pin::Pin, ptr};

use zerocopy::FromZeros;

use crate::alloc::strategy::PinStrategyHandle;

use super::{
  SliceDst, allocators::{AllocateError, OverflowedLayoutCalculation}, strategy::{Strategy, StrategyHandle, UninitStrategyHandleExt}
};

pub trait Allocator {
  type UnderlyingAllocateError: Error;

  async fn reserve_item<'allocator, S: Strategy, T: 'allocator>(
    &'allocator self,
    strategy: S,
  ) -> Result<S::UninitSizedHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>>;

  async fn reserve_dst<'allocator, S: Strategy, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    strategy: S,
    element_count: usize,
  ) -> Result<S::UninitSliceHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>>;

  async fn take_item<'allocator, S: Strategy, T: 'allocator>(
    &'allocator self,
    strategy: S,
    value: T,
  ) -> Result<S::SizedHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>> {
    let handle = self.reserve_item::<S, T>(strategy).await?;
    unsafe { handle.as_ptr().cast::<T>().write(value) };
    Ok(unsafe { UninitStrategyHandleExt::assume_init(handle) })
  }

  async fn pin_item<'allocator, S: Strategy, T: 'allocator>(
    &'allocator self,
    strategy: S,
    value: T,
  ) -> Result<Pin<S::SizedHandle<'allocator, T>>, AllocateError<Self::UnderlyingAllocateError>>
  where
    S::SizedHandle<'allocator, T>: PinStrategyHandle<T>,
  {
    Ok(self.take_item(strategy, value).await?.into_pin())
  }

  async fn take_from_iter<'allocator, S: Strategy, T: 'allocator>(
    &'allocator self,
    strategy: S,
    iterator: impl TrustedLen<Item = T>,
  ) -> Result<S::SliceHandle<'allocator, [T]>, AllocateError<Self::UnderlyingAllocateError>> {
    let Some(length) = iterator.size_hint().1 else {
      return Err(OverflowedLayoutCalculation.into());
    };

    let handle = self.reserve_dst::<S, [T]>(strategy, length).await?;

    let ptr = handle.as_ptr() as *mut T;
    let ptr = ptr;
    for (index, value) in iterator.enumerate() {
      unsafe { ptr.add(index).write(value) };
    }

    let handle = unsafe { UninitStrategyHandleExt::assume_init(handle) };

    Ok(handle)
  }

  async fn take_from_zeros<'allocator, S: Strategy, T: SliceDst + FromZeros + ?Sized + 'allocator>(
    &'allocator self,
    strategy: S,
    element_count: usize,
  ) -> Result<S::SliceHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>>
  where
    T::Element: FromZeros,
  {
    let handle = self.reserve_dst::<S, T>(strategy, element_count).await?;
    unsafe {
      let ptr = handle.as_ptr();
      ptr.cast::<T::Header>().write_bytes(0, 1);
      let (ptr, _) = ptr.to_raw_parts();
      T::addr_of_slice(ptr::from_raw_parts_mut(ptr, element_count)).cast::<T::Element>().write_bytes(0, element_count);
    };
    Ok(unsafe { UninitStrategyHandleExt::assume_init(handle) })
  }
}
