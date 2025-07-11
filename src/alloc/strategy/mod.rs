mod rc;
mod unique;

pub use rc::*;
pub use unique::*;

use core::{
  alloc::Layout, marker::PhantomInvariantLifetime, mem::MaybeUninit, pin::Pin, ptr::{NonNull, Pointee}
};

use super::{FreeVtable, SliceDst, UnsizedMaybeUninit};

#[inline]
fn assert_alignment<T: ?Sized>(data_ptr: *mut T) {
  let layout = unsafe { Layout::for_value_raw(data_ptr) };
  assert!(
    data_ptr.addr() as usize & (layout.align() - 1) == 0,
    "pointer was unaligned"
  );
}

pub trait Strategy {
  /// The data stored in the allocation.
  type Data<'a, T: ?Sized + 'a>: ?Sized;
  /// The handle that references the data.
  type Handle<'a, T: ?Sized + 'a>: StrategyHandle<'a, T>;

  /// Safety: data_ptr must be aligned and point to valid memory
  unsafe fn initialize_data<'a, T: ?Sized + 'a>(free_vtable: FreeVtable<'a>, data_ptr: *mut Self::Data<'a, T>);

  /// Safety: data_ptr must be aligned and point to valid memory
  fn construct_handle_sized<'a, T: 'a>(
    ptr: NonNull<Self::Data<'a, MaybeUninit<T>>>,
  ) -> Self::Handle<'a, MaybeUninit<T>>;

  /// Safety: data_ptr must be aligned and point to valid memory
  fn construct_handle_slice<'a, T: SliceDst + ?Sized + 'a>(
    ptr: NonNull<Self::Data<'a, UnsizedMaybeUninit<T>>>,
  ) -> Self::Handle<'a, UnsizedMaybeUninit<T>>;
}

pub trait StrategyHandle<'a, T: ?Sized + 'a>: Sized {
  type Cast<U: ?Sized + 'a>: StrategyHandle<'a, U>;

  fn as_value_ptr(this: &Self) -> *mut T;

  unsafe fn cast<U: ?Sized, M>(this: Self) -> Self::Cast<U>
  where
    T: Pointee<Metadata = M>,
    U: Pointee<Metadata = M>;
}

pub(super) type StrategyVariance<'t> = PhantomInvariantLifetime<'t>;

pub trait UninitStrategyHandleExt<'a, T: ?Sized + 'a>: StrategyHandle<'a, T> {
  type Init: 'a;

  unsafe fn assume_init(this: Self) -> Self::Init;
}

pub trait PinStrategyHandle<'a, T: ?Sized + 'a>: StrategyHandle<'a, T> {
  fn into_pin(self) -> Pin<Self>;
}
