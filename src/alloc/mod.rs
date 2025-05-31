pub mod dst_allocator;
pub mod item_allocator;

use core::{mem::MaybeUninit, ptr::NonNull};

pub trait Strategy {
  type Data; // stored in allocation
  type Handle<'a, T: ?Sized + 'a>: StrategyHandle<T>; // reference to allocation

  fn new_data() -> Self::Data;
  fn construct_handle<'a, T: ?Sized>(ptr: NonNull<T>) -> Self::Handle<'a, T>;
}

pub trait StrategyHandle<T: ?Sized>: Sized {
  fn as_ptr(&self) -> NonNull<T>;
  fn into_ptr(self) -> NonNull<T> { self.as_ptr() }
  unsafe fn from_ptr(ptr: NonNull<T>) -> Self;
}

pub trait AssumeInitHandleExt<T, U, V: StrategyHandle<U>>: StrategyHandle<T> {
  unsafe fn assume_init(self) -> V;
}

impl<T, U: StrategyHandle<MaybeUninit<T>>, V: StrategyHandle<T>> AssumeInitHandleExt<MaybeUninit<T>, T, V> for U {
  unsafe fn assume_init(self) -> V {
    unsafe { V::from_ptr(NonNull::new(self.into_ptr().as_ptr().cast()).unwrap()) }
  }
}
