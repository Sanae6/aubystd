use core::{
  alloc::Layout, cell::Cell, fmt::{self, Debug, Display}, marker::{CoercePointee, variance}, mem::MaybeUninit, ops::Deref, ptr::{self, Pointee}
};

use crate::alloc::{
  UnsizedMaybeUninit, strategy::{StrategyVariance, assert_alignment}
};

use super::{FreeVtable, SliceDst, Strategy, StrategyHandle, UninitStrategyHandleExt};

#[derive(Default)]
pub struct RcStrategy;
pub const RC: RcStrategy = RcStrategy;

#[doc(hidden)]
#[derive(SliceDst)]
#[repr(C)]
pub struct RcData<'a, T: ?Sized> {
  free_vtable: FreeVtable<'a>,
  ref_count: Cell<usize>,
  value: T,
}

impl Strategy for RcStrategy {
  type Data<'a, T: ?Sized + 'a> = RcData<'a, T>;
  type Handle<'a, T: ?Sized + 'a> = Rc<'a, T>;

  unsafe fn initialize_data<'a, T: ?Sized + 'a>(free_vtable: FreeVtable<'a>, data_ptr: *mut RcData<'a, T>) {
    assert_alignment(data_ptr);
    unsafe {
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);
    }
  }

  fn construct_handle_sized<'a, T: 'a>(
    ptr: ptr::NonNull<RcData<'a, MaybeUninit<T>>>,
  ) -> Self::Handle<'a, MaybeUninit<T>> {
    Rc(ptr, Default::default())
  }

  fn construct_handle_slice<'a, T: SliceDst + ?Sized + 'a>(
    ptr: ptr::NonNull<RcData<'a, UnsizedMaybeUninit<T>>>,
  ) -> Self::Handle<'a, UnsizedMaybeUninit<T>> {
    Rc(ptr, Default::default())
  }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct Rc<'a, T: ?Sized + 'a>(ptr::NonNull<RcData<'a, T>>, StrategyVariance<'a>);

impl<'a, T: ?Sized> StrategyHandle<'a, T> for Rc<'a, T> {
  type Cast<U: ?Sized + 'a> = Rc<'a, U>;

  fn as_value_ptr(this: &Self) -> *mut T {
    unsafe { (&raw mut (*this.0.as_ptr()).value) }
  }

  // Casts the smart pointer to `U`
  unsafe fn cast<U: ?Sized, M>(this: Self) -> Rc<'a, U>
  where
    T: Pointee<Metadata = M>,
    U: Pointee<Metadata = M>,
  {
    unsafe {
      assert_eq!(
        Layout::for_value_raw(this.0.as_ptr() as _),
        Layout::for_value_raw(this.0.as_ptr() as _)
      );
    }

    let (ptr, metadata) = this.0.to_raw_parts();
    let new_value = ptr::NonNull::from_raw_parts(ptr as _, metadata);
    Rc(new_value, variance())
  }
}

impl<'a, T: ?Sized> AsRef<T> for Rc<'a, T> {
  fn as_ref(&self) -> &T {
    // Safety: you can never get a mutable reference to the value
    unsafe { &self.0.as_ref().value }
  }
}

impl<'a, T: ?Sized> Deref for Rc<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    self.as_ref()
  }
}

impl<'a, T: Sized> Clone for Rc<'a, T> {
  fn clone(&self) -> Self {
    let Self(ptr, phantom) = self;
    Self(ptr.clone(), phantom.clone())
  }
}

impl<'a, T: ?Sized> Drop for Rc<'a, T> {
  fn drop(&mut self) {
    // Safety: you can never get a mutable reference to the data
    let data = unsafe { &*self.0.as_ptr() };

    let ref_count = data.ref_count.get();

    if ref_count == usize::MAX {
      return;
    }

    data.ref_count.set(ref_count.saturating_sub(1));
    if ref_count == 0 {
      let layout = Layout::for_value(self.as_ref());
      // reading
      unsafe { (&raw const data.free_vtable).read().free(self.0, layout) };
    }
  }
}

impl<'a, T: Debug + ?Sized> Debug for Rc<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.deref().fmt(f)
  }
}

impl<'a, T: Display + ?Sized> Display for Rc<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.deref().fmt(f)
  }
}

impl<'a, T> UninitStrategyHandleExt<'a, MaybeUninit<T>> for Rc<'a, MaybeUninit<T>> {
  type Init = Rc<'a, T>;

  unsafe fn assume_init(this: Self) -> Self::Init {
    unsafe { Self::cast(this) }
  }
}

impl<'a, T: SliceDst + ?Sized> UninitStrategyHandleExt<'a, UnsizedMaybeUninit<T>> for Rc<'a, UnsizedMaybeUninit<T>> {
  type Init = Rc<'a, T>;

  unsafe fn assume_init(this: Self) -> Self::Init {
    unsafe { Self::cast(this) }
  }
}

#[cfg(test)]
pub mod tests {
  use core::cell::Cell;

  use crate::alloc::{
    allocator::test_arena, strategy::{RcData, RcStrategy}
  };

  #[pollster::test]
  async fn allocate() {
    let arena = test_arena::<{ size_of::<RcData<Cell<u32>>>() }>();
    let handle = arena.take::<RcStrategy>(Cell::new(42)).await.unwrap();
    assert_eq!(handle.get(), 42);
    let second_handle = handle.clone();
    assert_eq!(second_handle.get(), 42);
    second_handle.set(16);
    assert_eq!(handle.get(), 16);
  }
}
