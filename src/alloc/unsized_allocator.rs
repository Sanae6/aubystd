use core::{error::Error, iter::TrustedLen, ptr};

use zerocopy::FromZeros;

use super::{
  SliceDst, allocators::{AllocateError, OverflowedLayoutCalculation}, strategy::{Strategy, StrategyHandle, UninitStrategyHandleExt}
};

pub trait Allocator<S: Strategy> {
  type UnderlyingAllocateError: Error;

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
  ) -> Result<S::UninitSizedHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>>;

  async fn reserve_dst<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::UninitSliceHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>>;

  async fn take_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::SizedHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>> {
    let handle = self.reserve_item().await?;
    unsafe { handle.as_ptr().cast::<T>().write(value) };
    Ok(unsafe { UninitStrategyHandleExt::assume_init(handle) })
  }

  async fn take_from_iter<'allocator, T: 'allocator>(
    &'allocator self,
    iterator: impl TrustedLen<Item = T>,
  ) -> Result<S::SliceHandle<'allocator, [T]>, AllocateError<Self::UnderlyingAllocateError>> {
    let Some(length) = iterator.size_hint().1 else {
      return Err(OverflowedLayoutCalculation.into());
    };

    let handle = self.reserve_dst::<[T]>(length).await?;

    let ptr = handle.as_ptr() as *mut T;
    let ptr = ptr;
    for (index, value) in iterator.enumerate() {
      unsafe { ptr.add(index).write(value) };
    }

    let handle = unsafe { UninitStrategyHandleExt::assume_init(handle) };

    Ok(handle)
  }

  async fn take_from_zeros<'allocator, T: SliceDst + FromZeros + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::SliceHandle<'allocator, T>, AllocateError<Self::UnderlyingAllocateError>>
  where
    T::Element: FromZeros,
  {
    let handle = self.reserve_dst::<T>(element_count).await?;
    unsafe {
      let ptr = handle.as_ptr();
      ptr.cast::<T::Header>().write_bytes(0, 1);
      let (ptr, _) = ptr.to_raw_parts();
      T::addr_of_slice(ptr::from_raw_parts_mut(ptr, element_count)).cast::<T::Element>().write_bytes(0, element_count);
    };
    Ok(unsafe { UninitStrategyHandleExt::assume_init(handle) })
  }
}
