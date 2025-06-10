use core::{
  alloc::Layout, cell::Cell, fmt::{self, Debug, Display}, marker::{CoercePointee, PhantomCovariantLifetime}, mem::MaybeUninit, ops::Deref, ptr
};

use crate::alloc::UnsizedMaybeUninit;

use super::{FreeVtable, SliceDst, Strategy, StrategyDataPtr, StrategyHandle, UninitStrategyHandleExt};

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
  type SizedData<'allocator, T: 'allocator> = RcData<'allocator, T>;
  type SliceData<'allocator, T: SliceDst + ?Sized + 'allocator> = RcData<'allocator, T>;
  type SizedHandle<'allocator, T: 'allocator> = Rc<'allocator, T>;
  type SliceHandle<'allocator, T: SliceDst + ?Sized + 'allocator> = Rc<'allocator, T>;
  type UninitSizedHandle<'allocator, T: 'allocator> = Rc<'allocator, MaybeUninit<T>>;
  type UninitSliceHandle<'allocator, T: SliceDst + ?Sized + 'allocator> = Rc<'allocator, UnsizedMaybeUninit<T>>;

  fn initialize_data_sized<'allocator, T: 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut RcData<'allocator, T>,
  ) {
    unsafe {
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);
    }
  }

  fn initialize_data_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SliceData<'allocator, T>,
  ) {
    unsafe {
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);
    }
  }

  fn construct_handle_sized<'allocator, T: 'allocator>(
    ptr: ptr::NonNull<RcData<'allocator, MaybeUninit<T>>>,
  ) -> Self::UninitSizedHandle<'allocator, T> {
    Rc(ptr, Default::default())
  }

  fn construct_handle_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    ptr: ptr::NonNull<RcData<'allocator, UnsizedMaybeUninit<T>>>,
  ) -> Self::UninitSliceHandle<'allocator, T> {
    Rc(ptr, Default::default())
  }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct Rc<'allocator, T: ?Sized + 'allocator>(
  ptr::NonNull<RcData<'allocator, T>>,
  PhantomCovariantLifetime<'allocator>,
);

impl<'allocator, T: ?Sized> StrategyHandle<T> for Rc<'allocator, T> {
  type Cast<'cast, U: ?Sized + 'cast> = Rc<'cast, U>;

  fn as_ptr(&self) -> *mut T {
    unsafe { (&raw mut (*self.0.as_ptr()).value) }
  }

  fn into_strategy_data_ptr(self) -> StrategyDataPtr<T> {
    let strategy_data_ptr = StrategyDataPtr {
      strategy_data_ptr: unsafe { ptr::NonNull::new_unchecked(self.0.as_ptr() as *mut _) },
      value: self.as_ptr(),
    };

    core::mem::forget(self);

    strategy_data_ptr
  }

  unsafe fn from_strategy_data_ptr(
    StrategyDataPtr {
      strategy_data_ptr: start_ptr,
      value: value_ptr,
    }: StrategyDataPtr<T>,
  ) -> Self {
    let ptr = ptr::from_raw_parts_mut(start_ptr.as_ptr(), value_ptr.to_raw_parts().1);
    Rc(unsafe { ptr::NonNull::new_unchecked(ptr) }, Default::default())
  }
}

impl<'a, T: ?Sized> AsRef<T> for Rc<'a, T> {
  fn as_ref(&self) -> &T {
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
    let data = unsafe { &*self.0.as_ptr() };

    let ref_count = data.ref_count.get();

    if ref_count == usize::MAX {
      return;
    }

    data.ref_count.set(ref_count.saturating_sub(1));
    if ref_count == 0 {
      let layout = Layout::for_value(self.as_ref());
      unsafe { (&raw mut (*self.0.as_ptr()).free_vtable).read().free(self.0, layout) };
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

impl<'a, T> UninitStrategyHandleExt<MaybeUninit<T>> for Rc<'a, MaybeUninit<T>> {
  type Init = Rc<'a, T>;

  unsafe fn assume_init(self) -> Self::Init {
    let StrategyDataPtr {
      strategy_data_ptr: strategy_data,
      value,
    } = Self::into_strategy_data_ptr(self);
    let (ptr, size) = value.to_raw_parts();
    let value = ptr::from_raw_parts_mut(ptr, size);
    unsafe {
      Rc::from_strategy_data_ptr(StrategyDataPtr {
        strategy_data_ptr: strategy_data,
        value,
      })
    }
  }
}

impl<'a, T: SliceDst + ?Sized> UninitStrategyHandleExt<UnsizedMaybeUninit<T>> for Rc<'a, UnsizedMaybeUninit<T>> {
  type Init = Rc<'a, T>;

  unsafe fn assume_init(self) -> Self::Init {
    let StrategyDataPtr {
      strategy_data_ptr: strategy_data,
      value,
    } = Self::into_strategy_data_ptr(self);
    let (ptr, size) = value.to_raw_parts();
    let value = ptr::from_raw_parts_mut(ptr, size);
    unsafe {
      Rc::from_strategy_data_ptr(StrategyDataPtr {
        strategy_data_ptr: strategy_data,
        value,
      })
    }
  }
}

#[cfg(test)]
pub mod tests {
  use core::cell::Cell;

  use crate::alloc::{
    allocators::test_arena, strategy::{RC, RcData}
  };

  #[pollster::test]
  async fn allocate() {
    let arena = test_arena::<{ size_of::<RcData<Cell<u32>>>() }>();
    let handle = arena.take_item(RC, Cell::new(42)).await.unwrap();
    assert_eq!(handle.get(), 42);
    let second_handle = handle.clone();
    assert_eq!(second_handle.get(), 42);
    second_handle.set(16);
    assert_eq!(handle.get(), 16);
  }
}
