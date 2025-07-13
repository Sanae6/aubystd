mod rc;
mod unique;

pub use rc::*;
pub use unique::*;

use core::{
  alloc::Layout, marker::PhantomInvariantLifetime, mem::MaybeUninit, pin::Pin, ptr::{NonNull, Pointee}
};

use crate::alloc::UnsizedMaybeUninit;

use super::{FreeVtable, SliceDst};

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
  type Data<'a, T: ?Sized + Pointee + 'a>: ?Sized + Pointee<Metadata = T::Metadata>;
  /// The handle that references the data.
  type Handle<'a, T: ?Sized + 'a>: StrategyHandle<'a, T>;
  /// A second handle associated type to simplify generic code.
  type UninitHandle<'a, T: UninitType + ?Sized + 'a>: UninitStrategyHandleExt<'a, T, Init = Self::Handle<'a, T::Init>>;

  /// Safety: data_ptr must be aligned and point to valid memory
  unsafe fn initialize_data<'a, T: ?Sized + 'a>(free_vtable: FreeVtable<'a>, data_ptr: *mut Self::Data<'a, T>);

  /// Safety: data_ptr must be aligned and point to valid memory
  fn construct_handle<'a, T: UninitType + ?Sized + 'a>(ptr: NonNull<Self::Data<'a, T>>) -> Self::UninitHandle<'a, T>;
}

pub trait StrategyHandle<'a, T: ?Sized + Pointee + 'a>: Sized {
  type Cast<U: ?Sized + 'a>: StrategyHandle<'a, U>;

  fn as_value_ptr(this: &Self) -> *mut T;
  unsafe fn from_value_ptr(ptr: *mut T) -> Self;
  fn into_value_ptr(this: Self) -> *mut T {
    let ptr = Self::as_value_ptr(&this);
    core::mem::forget(this);
    ptr
  }

  unsafe fn cast<U: ?Sized + Pointee<Metadata = T::Metadata>>(medadata: T::Metadata, this: Self) -> Self::Cast<U>;
}

pub(super) type StrategyVariance<'t> = PhantomInvariantLifetime<'t>;

pub trait UninitStrategyHandleExt<'a, T: ?Sized + 'a>: StrategyHandle<'a, T> {
  type Init: 'a;

  unsafe fn assume_init(this: Self) -> Self::Init;
}

pub trait UninitType: Pointee {
  type Init: ?Sized + Pointee<Metadata = Self::Metadata>;
}
impl<T> UninitType for MaybeUninit<T> {
  type Init = T;
}
impl<T: SliceDst + ?Sized> UninitType for UnsizedMaybeUninit<T> {
  type Init = T;
}

pub trait PinStrategyHandle<'a, T: ?Sized + 'a>: StrategyHandle<'a, T> {
  fn into_pin(self) -> Pin<Self>;
}
