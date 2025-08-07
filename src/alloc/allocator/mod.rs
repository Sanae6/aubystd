pub mod arena;
pub mod foreign;

use core::{
  alloc::{Layout, LayoutError},
  error::Error,
  mem::MaybeUninit,
  pin::Pin,
  ptr,
};

#[doc(inline)]
pub use arena::*;
#[doc(inline)]
pub use foreign::*;

use thiserror::Error;
use zerocopy::FromZeros;

use crate::alloc::{UnsizedMaybeUninit, strategy::Strategy};

use super::SliceDst;

#[derive(Debug, Error)]
#[error("ran out of memory")]
pub struct OutOfMemory;

pub trait Allocator<'s, T: 's> {
  type Error: Error;

  /// Allocates an uninitialized handle
  async fn reserve_item<S: Strategy>(
    &'s self,
  ) -> Result<S::UninitHandle<'s, MaybeUninit<T>>, Self::Error>
  where
    S::Data<'s, MaybeUninit<T>>: Sized;

  async fn take<S: Strategy>(&'s self, value: T) -> Result<S::Handle<'s, T>, Self::Error>
  where
    S::Data<'s, MaybeUninit<T>>: Sized,
  {
    let item = self.reserve_item::<S>().await?;
    //
    unsafe {
      S::UninitHandle::as_value_ptr(&item)
        .cast::<T>()
        .write(value)
    };
    // Safety: item was initialized above
    Ok(unsafe { S::UninitHandle::assume_init(item).into() })
  }

  async fn pin<S: Strategy>(&'s self, value: T) -> Result<Pin<S::Handle<'s, T>>, Self::Error>
  where
    S::Data<'s, MaybeUninit<T>>: Sized,
    S::Handle<'s, T>: PinStrategyHandle<'s, T>,
  {
    Ok(self.take::<S>(value).await?.into_pin())
  }
}

pub trait SliceAllocator<'s, T: SliceDst + ?Sized + 's> {
  type Error: Error;

  async fn reserve_slice<S: Strategy>(
    &'s self,
    length: usize,
  ) -> Result<S::UninitHandle<'s, UnsizedMaybeUninit<T>>, Self::Error>
  where
    S::Data<'s, UnsizedMaybeUninit<T>>: SliceDst;

  async fn from_zeros<S: Strategy>(&'s self, length: usize) -> Result<S::Handle<'s, T>, Self::Error>
  where
    T::Header: FromZeros,
    T::Element: FromZeros,
    S::Data<'s, UnsizedMaybeUninit<T>>: SliceDst,
  {
    let slice = self.reserve_slice::<S>(length).await?;
    unsafe {
      let ptr = S::UninitHandle::as_value_ptr(&slice);
      ptr.cast::<T::Header>().write_bytes(0, 1);
      let (ptr, _) = ptr.to_raw_parts();
      T::addr_of_slice(ptr::from_raw_parts_mut(ptr, length))
        .cast::<T::Element>()
        .write_bytes(0, length);
    };
    Ok(unsafe { S::UninitHandle::assume_init(slice) })
  }
}

pub trait LayoutAllocator {
  type Error: Error;

  async fn reserve_layout<'s, S: Strategy>(
    &'s self,
    layout: Layout,
  ) -> Result<S::Handle<'s, [MaybeUninit<u8>]>, Self::Error>
  where
    S::Data<'s, ()>: Sized;
  // S::Data<'s, [MaybeUninit<u8>]>: Pointee<Metadata = usize>;
}

pub fn calculate_layout_for_dst<T: SliceDst + ?Sized>(
  element_count: usize,
) -> Result<Layout, LayoutError> {
  let header = Layout::new::<T::Header>();
  let array = Layout::array::<T::Element>(element_count)?;
  Layout::extend(&header, array).map(|tuple| tuple.0.pad_to_align())

  // would be nice to rely on for_value_raw, but it has safety issues that can't be ignored if layout calc overflows
  // Ok(unsafe { Layout::for_value_raw(ptr) })
}
