mod rc;
mod unique;

pub use rc::*;
pub use unique::*;

use core::{
  mem::MaybeUninit, pin::Pin, ptr::{NonNull, Pointee}
};

use super::{FreeVtable, SliceDst, UnsizedMaybeUninit};

pub trait Strategy {
  // stored in allocation
  type SizedData<'allocator, T: 'allocator>;
  type SliceData<'allocator, T: SliceDst + ?Sized + 'allocator>: SliceDst + ?Sized + Pointee<Metadata = usize>;
  // reference to allocation
  type SizedHandle<'allocator, T: 'allocator>: StrategyHandle<T>;
  type SliceHandle<'allocator, T: SliceDst + ?Sized + 'allocator>: StrategyHandle<T>;
  // uninitialized reference to allocation
  type UninitSizedHandle<'allocator, T: 'allocator>: StrategyHandle<MaybeUninit<T>>
    + UninitStrategyHandleExt<MaybeUninit<T>, Init = Self::SizedHandle<'allocator, T>>;
  type UninitSliceHandle<'allocator, T: SliceDst + ?Sized + 'allocator>: StrategyHandle<UnsizedMaybeUninit<T>>
    + UninitStrategyHandleExt<UnsizedMaybeUninit<T>, Init = Self::SliceHandle<'allocator, T>>;

  fn initialize_data_sized<'allocator, T: 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SizedData<'allocator, T>,
  );

  fn initialize_data_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SliceData<'allocator, T>,
  );

  fn construct_handle_sized<'allocator, T: 'allocator>(
    ptr: NonNull<Self::SizedData<'allocator, MaybeUninit<T>>>,
  ) -> Self::UninitSizedHandle<'allocator, T>;

  fn construct_handle_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    ptr: NonNull<Self::SliceData<'allocator, UnsizedMaybeUninit<T>>>,
  ) -> Self::UninitSliceHandle<'allocator, T>;
}

pub trait StrategyHandle<T: ?Sized>: Sized {
  type Cast<'allocator, U: ?Sized + 'allocator>: StrategyHandle<U>;

  fn as_ptr(&self) -> *mut T;
  fn into_strategy_data_ptr(self) -> StrategyDataPtr<T>;
  unsafe fn from_strategy_data_ptr(ptr: StrategyDataPtr<T>) -> Self;
}

/// Represents the pointer to the strategy data that contains T.
/// This newtype is intended to indicate that the contained pointer is *not* simply a pointer T.
///
/// The reason for this type's existence is the generics issues of passing strategy data into [StrategyHandle] for
/// [StrategyHandle::into_strategy_data_ptr] and [StrategyHandle::from_strategy_data_ptr].
pub struct StrategyDataPtr<T: ?Sized> {
  strategy_data_ptr: NonNull<()>,
  value: *mut T,
}

impl<T: ?Sized> StrategyDataPtr<T> {
  pub fn as_strategy_data_ptr(&self) -> NonNull<()> {
    self.strategy_data_ptr
  }
  pub fn as_value_ptr(&self) -> *mut T {
    self.value
  }
  pub fn into_pair(self) -> (NonNull<()>, *mut T) {
    (self.strategy_data_ptr, self.value)
  }
  pub fn from_pair(strategy_data_ptr: NonNull<()>, value: *mut T) -> Self {
    Self {
      strategy_data_ptr,
      value,
    }
  }
}

pub trait UninitStrategyHandleExt<T: ?Sized>: StrategyHandle<T> {
  type Init;

  unsafe fn assume_init(self) -> Self::Init;
}

pub trait PinStrategyHandle<T: ?Sized>: StrategyHandle<T> {
  fn into_pin(self) -> Pin<Self>;
}
