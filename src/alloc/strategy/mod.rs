pub mod unique;

use core::ptr::{NonNull, Pointee};

use super::{unsized_allocator::SliceDst, free::FreeVtable};

pub trait Strategy {
  // Data types *must* be repr(C).
  type SizedData<'allocator, T: 'allocator>; // stored in allocation
  type SliceData<'allocator, T: SliceDst + ?Sized + 'allocator>: ?Sized + Pointee<Metadata = usize>; // stored in allocation
  type Handle<'allocator, T: ?Sized + 'allocator>: StrategyHandle<T>; // reference to allocation

  fn initialize_data_sized<'allocator, T: 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SizedData<'allocator, T>,
  ) -> *mut T;

  fn initialize_data_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SliceData<'allocator, T>,
  ) -> *mut T;

  fn construct_handle_sized<'allocator, T: 'allocator>(
    ptr: NonNull<Self::SizedData<'allocator, T>>,
  ) -> Self::Handle<'allocator, T>;

  fn construct_handle_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    ptr: NonNull<Self::SliceData<'allocator, T>>,
  ) -> Self::Handle<'allocator, T>;
}

pub trait StrategyHandle<T: ?Sized>: Sized {
  fn as_ptr(&self) -> NonNull<T>;
  fn into_strategy_data_ptr(self) -> StrategyDataPtr<T>;
  unsafe fn from_strategy_data_ptr(ptr: StrategyDataPtr<T>) -> Self;
}

/// Represents the pointer to the strategy data that contains T.
/// This newtype is intended to indicate that the contained pointer is *not* simply a pointer T.
///
/// The reason for this type's existence is the generics issues of passing [Strategy::Data] into [Strategy::Handle] for
/// [StrategyHandle::into_strategy_data_ptr] and [StrategyHandle::from_strategy_data_ptr].
pub struct StrategyDataPtr<T: ?Sized>(pub NonNull<T>);
