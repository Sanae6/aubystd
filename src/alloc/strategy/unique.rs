use core::{
  alloc::Layout, borrow::{Borrow, BorrowMut}, fmt::{self, Debug, Display}, marker::{CoercePointee, variance}, mem::{forget, offset_of}, ops::{Deref, DerefMut, DerefPure}, pin::Pin, ptr::{self, Pointee}
};

use aubystd_macros::slice_dst;

use crate::alloc::strategy::{PinStrategyHandle, StrategyVariance, UninitType, assert_alignment};

use super::{FreeVtable, Strategy, StrategyHandle, UninitStrategyHandleExt};

#[derive(Default)]
pub struct UniqueStrategy;
pub const UNIQUE: UniqueStrategy = UniqueStrategy;

#[slice_dst(header = UniqueDataHeader)]
#[doc(hidden)]
#[repr(C)]
pub struct UniqueData<'a, T: ?Sized> {
  free_vtable: FreeVtable<'a>,
  value: T,
}

impl Strategy for UniqueStrategy {
  type Data<'a, T: ?Sized + 'a> = UniqueData<'a, T>;
  type Handle<'a, T: ?Sized + 'a> = Unique<'a, T>;
  type UninitHandle<'a, T: UninitType + ?Sized + 'a> = Unique<'a, T>;

  /// Safety: data_ptr must be aligned and point to valid memory
  unsafe fn initialize_data<'a, T: ?Sized + 'a>(free_vtable: FreeVtable<'a>, data_ptr: *mut Self::Data<'a, T>) {
    assert_alignment(data_ptr);
    // Safety: data_ptr is valid and properly aligned
    unsafe {
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);
    }
  }

  fn construct_handle<'a, T: ?Sized + 'a>(ptr: ptr::NonNull<UniqueData<'a, T>>) -> Self::Handle<'a, T> {
    Unique(ptr, variance())
  }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct Unique<'a, T: ?Sized + 'a>(ptr::NonNull<UniqueData<'a, T>>, StrategyVariance<'a>);

impl<'a, T> Unique<'a, T> {
  pub fn into_inner(self) -> T {
    unsafe {
      // Safety: ptr is valid, and we aren't dropping the value in-place
      let data = self.0.read();
      // Safety: ptr's layout is already known to be safe to use
      let layout = Layout::for_value_raw(self.0.as_ptr().cast_const());
      // Safety: upheld by previous statements and full ownership of `self`
      (&raw const (*self.0.as_ptr()).free_vtable).read().free(self.0, layout);
      data.value
    }
  }
}

impl<'a, T: ?Sized> Unique<'a, T> {
  pub fn into_pin(unique: Self) -> Pin<Self> {
    // safety comment from Box::into_pin
    // It's not possible to move or replace the insides of a `Pin<Unique<T>>`
    // when `T: !Unpin`, so it's safe to pin it directly without any
    // additional requirements.
    unsafe { Pin::new_unchecked(unique) }
  }
}

impl<'a, T: ?Sized + Pointee> StrategyHandle<'a, T> for Unique<'a, T> {
  type Cast<U: ?Sized + 'a> = Unique<'a, U>;

  fn as_value_ptr(this: &Self) -> *mut T {
    // safety: value is valid and properly aligned
    unsafe { (&raw mut (*this.0.as_ptr()).value) }
  }

  unsafe fn from_value_ptr(ptr: *mut T) -> Self {
    let (ptr, metadata) = unsafe { ptr.byte_sub(offset_of!(UniqueData<'static, ()>, value)) }.to_raw_parts();
    let ptr = ptr::NonNull::from_raw_parts(ptr::NonNull::new(ptr).unwrap(), metadata);
    Unique(ptr, variance())
  }

  // Casts the smart pointer to `U`
  unsafe fn cast<U: ?Sized + Pointee<Metadata = T::Metadata>>(metadata: T::Metadata, this: Self) -> Self::Cast<U> {
    let (ptr, _) = this.0.to_raw_parts();
    let new_value = ptr::NonNull::<UniqueData<'a, U>>::from_raw_parts(ptr as _, metadata);
    unsafe {
      assert_eq!(
        Layout::for_value_raw::<UniqueData<'a, T>>(this.0.as_ptr().cast_const()),
        Layout::for_value_raw::<UniqueData<'a, U>>(new_value.as_ptr().cast_const())
      );
    }

    forget(this);

    Unique(new_value, variance())
  }
}

impl<'a, T: ?Sized> Borrow<T> for Unique<'a, T> {
  fn borrow(&self) -> &T {
    unsafe { &self.0.as_ref().value }
  }
}

impl<'a, T: ?Sized> BorrowMut<T> for Unique<'a, T> {
  fn borrow_mut(&mut self) -> &mut T {
    unsafe { &mut self.0.as_mut().value }
  }
}

impl<'a, T: ?Sized> AsRef<T> for Unique<'a, T> {
  fn as_ref(&self) -> &T {
    self.borrow()
  }
}

impl<'a, T: ?Sized> AsMut<T> for Unique<'a, T> {
  fn as_mut(&mut self) -> &mut T {
    self.borrow_mut()
  }
}

impl<'a, T: ?Sized> Deref for Unique<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &self.0.as_ref().value }
  }
}

impl<'a, T: ?Sized> DerefMut for Unique<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut self.0.as_mut().value }
  }
}

unsafe impl<'a, T: ?Sized> DerefPure for Unique<'a, T> {}

impl<'a, T: ?Sized> Drop for Unique<'a, T> {
  fn drop(&mut self) {
    unsafe {
      // Safety: ptr is valid, aligned, non-null, and only one reference to its value is held.
      self.0.drop_in_place();
      // Safety: ptr's layout is already known to be safe to use
      let layout = Layout::for_value_raw(self.0.as_ptr() as *const _);
      (&raw mut (*self.0.as_ptr()).free_vtable).read().free(self.0, layout);
    }
  }
}

impl<'a, T: Debug + ?Sized> Debug for Unique<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.as_ref().fmt(f)
  }
}

impl<'a, T: Display + ?Sized> Display for Unique<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.as_ref().fmt(f)
  }
}

impl<'a, U: UninitType + ?Sized> UninitStrategyHandleExt<'a, U> for Unique<'a, U> {
  type Init = Unique<'a, U::Init>;

  unsafe fn assume_init(this: Self) -> Self::Init {
    unsafe { Self::cast(this.0.to_raw_parts().1, this) }
  }
}

impl<'a, T: ?Sized> Unpin for Unique<'a, T> {}

impl<'a, T: ?Sized> PinStrategyHandle<'a, T> for Unique<'a, T> {
  fn into_pin(self) -> Pin<Self> {
    // See alloc::boxed::Box::into_pin
    unsafe { Pin::new_unchecked(self) }
  }
}
