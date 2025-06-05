mod rc;
mod unique;

pub use rc::*;
pub use unique::*;

use core::{
  mem::MaybeUninit, ptr::{NonNull, Pointee}
};

use super::{FreeVtable, SliceDst, UnsizedMaybeUninit};

pub trait Strategy {
  type SizedData<'allocator, T: 'allocator>; // stored in allocation
  type SliceData<'allocator, T: SliceDst + ?Sized + 'allocator>: SliceDst + ?Sized + Pointee<Metadata = usize>; // stored in allocation
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
/// The reason for this type's existence is the generics issues of passing [Strategy::Data] into [Strategy::Handle] for
/// [StrategyHandle::into_strategy_data_ptr] and [StrategyHandle::from_strategy_data_ptr].
pub struct StrategyDataPtr<T: ?Sized> {
  pub strategy_data: NonNull<()>,
  pub value: *mut T,
}

pub trait UninitStrategyHandleExt<T: ?Sized>: StrategyHandle<T> {
  type Init;

  unsafe fn assume_init(self) -> Self::Init;
}
