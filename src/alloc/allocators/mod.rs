mod foreign;

use core::{alloc::Layout, error::Error};

pub use foreign::*;

use thiserror::Error;

use super::SliceDst;

#[derive(Debug, Error)]
#[error("overflowed while attempting to calculate layout")]
pub struct OverflowedLayoutCalculation;
#[derive(Debug, Error)]
#[error("allocator is out of memory")]
pub struct OutOfMemory;

#[derive(Debug, Error)]
pub enum AllocateError<T: Error> {
  #[error("{0}")]
  OverflowedLayoutCalculation(#[from] OverflowedLayoutCalculation),
  #[error("{0}")]
  OutOfMemory(#[from] OutOfMemory),
  #[error("{0}")]
  Underlying(T),
}

pub fn calculate_layout_for_dst<T: SliceDst + ?Sized>(
  element_count: usize,
) -> Result<Layout, OverflowedLayoutCalculation> {
  let header = Layout::new::<T::Header>();
  let array = Layout::array::<T::Element>(element_count).map_err(|_| OverflowedLayoutCalculation)?;
  Layout::extend(&header, array).map(|tuple| tuple.0).map_err(|_| OverflowedLayoutCalculation)

  // would be nice to rely on for_value_raw, but it has safety issues that can't be ignored if layout calc overflows
  // Ok(unsafe { Layout::for_value_raw(ptr) })
}
