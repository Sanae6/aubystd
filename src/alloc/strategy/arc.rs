use core::{
  alloc::Layout,
  fmt::{self, Debug, Display},
  marker::{CoercePointee, variance},
  mem::{forget, offset_of},
  ops::Deref,
  ptr::{self, Pointee},
  sync::atomic::{AtomicUsize, Ordering},
};

use aubystd_macros::slice_dst;

use crate::alloc::strategy::{StrategyVariance, UninitType, assert_alignment};

use super::{FreeVtable, Strategy, StrategyHandle, UninitStrategyHandleExt};

#[derive(Default)]
pub struct ArcStrategy;

#[slice_dst(header = ArcDataHeader)]
#[doc(hidden)]
#[repr(C)]
pub struct ArcData<'a, T: ?Sized> {
  free_vtable: FreeVtable<'a>,
  ref_count: AtomicUsize,
  value: T,
}

impl Strategy for ArcStrategy {
  type Data<'a, T: ?Sized + 'a> = ArcData<'a, T>;
  type Handle<'a, T: ?Sized + 'a> = Arc<'a, T>;
  type UninitHandle<'a, T: UninitType + ?Sized + 'a> = Arc<'a, T>;

  unsafe fn initialize_data<'a, T: ?Sized + 'a>(
    free_vtable: FreeVtable<'a>,
    data_ptr: *mut ArcData<'a, T>,
  ) {
    assert_alignment(data_ptr);
    unsafe {
      (&raw mut (*data_ptr).ref_count).write(AtomicUsize::new(1));
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);
    }
  }

  fn construct_handle<'a, T: ?Sized + 'a>(
    ptr: ptr::NonNull<ArcData<'a, T>>,
  ) -> Self::Handle<'a, T> {
    Arc(ptr, Default::default())
  }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct Arc<'a, T: ?Sized + 'a>(ptr::NonNull<ArcData<'a, T>>, StrategyVariance<'a>);

impl<'a, T: ?Sized + Pointee> StrategyHandle<'a, T> for Arc<'a, T> {
  type Cast<U: ?Sized + 'a> = Arc<'a, U>;

  fn as_value_ptr(this: &Self) -> *mut T {
    // safety: value is valid and properly aligned
    unsafe { (&raw mut (*this.0.as_ptr()).value) }
  }

  unsafe fn from_value_ptr(ptr: *mut T) -> Self {
    let (ptr, metadata) =
      unsafe { ptr.byte_sub(offset_of!(ArcData<'static, ()>, value)) }.to_raw_parts();
    let ptr = ptr::NonNull::from_raw_parts(ptr::NonNull::new(ptr).unwrap(), metadata);
    Arc(ptr, variance())
  }

  // Casts the smart pointer to `U`
  unsafe fn cast<U: ?Sized + Pointee<Metadata = T::Metadata>>(
    metadata: T::Metadata,
    this: Self,
  ) -> Arc<'a, U> {
    let (ptr, _) = this.0.to_raw_parts();
    let new_value = ptr::NonNull::<ArcData<'a, U>>::from_raw_parts(ptr as _, metadata);
    unsafe {
      assert_eq!(
        Layout::for_value_raw::<ArcData<'a, T>>(this.0.as_ptr().cast_const()),
        Layout::for_value_raw::<ArcData<'a, U>>(new_value.as_ptr().cast_const())
      );
    }

    forget(this);

    Arc(new_value, variance())
  }
}

impl<'a, T: ?Sized> AsRef<T> for Arc<'a, T> {
  fn as_ref(&self) -> &T {
    // Safety: you can never get a mutable reference to the value
    unsafe { &self.0.as_ref().value }
  }
}

impl<'a, T: ?Sized> Deref for Arc<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    self.as_ref()
  }
}

impl<'a, T: Sized> Clone for Arc<'a, T> {
  fn clone(&self) -> Self {
    if unsafe { self.0.as_ref() }
      .ref_count
      .fetch_add(1, Ordering::Relaxed)
      == usize::MAX
    {
      panic!("too many references!");
    }
    Self(self.0.clone(), variance())
  }
}

impl<'a, T: ?Sized> Drop for Arc<'a, T> {
  fn drop(&mut self) {
    // Safety: you can never get a mutable reference to the data
    let data = unsafe { &*self.0.as_ptr() };

    let ref_count = data.ref_count.fetch_sub(1, Ordering::Release);

    if ref_count == usize::MAX {
      return;
    }

    if ref_count == 1 {
      let layout = Layout::for_value(self.as_ref());
      // reading
      unsafe { (&raw const data.free_vtable).read().free(self.0, layout) };
    }
  }
}

impl<'a, T: Debug + ?Sized> Debug for Arc<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.deref().fmt(f)
  }
}

impl<'a, T: Display + ?Sized> Display for Arc<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.deref().fmt(f)
  }
}

impl<'a, U: UninitType + ?Sized> UninitStrategyHandleExt<'a, U> for Arc<'a, U> {
  type Init = Arc<'a, U::Init>;

  unsafe fn assume_init(this: Self) -> Self::Init {
    unsafe { Self::cast(this.0.to_raw_parts().1, this) }
  }
}

#[cfg(test)]
#[cfg(feature = "libc")]
pub mod tests {
  use core::cell::Cell;

  use crate::{alloc::strategy::ArcStrategy, test_arena};

  #[pollster::test]
  async fn allocate() {
    let arena = test_arena!(ArcStrategy, 1).await;
    let handle = arena.take::<ArcStrategy>(Cell::new(42)).await.unwrap();
    assert_eq!(handle.get(), 42);
    let second_handle = handle.clone();
    assert_eq!(second_handle.get(), 42);
    second_handle.set(16);
    assert_eq!(handle.get(), 16);
  }
}
