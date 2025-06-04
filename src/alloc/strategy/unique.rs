use core::{
  fmt::{self, Debug, Display}, marker::{CoercePointee, PhantomCovariantLifetime}, ops::{Deref, DerefMut}, ptr
};

use crate::alloc::UnsizedMaybeUninit;

use super::{FreeVtable, SliceDst, Strategy, StrategyDataPtr, StrategyHandle, UninitStrategyHandleExt};

pub struct Unique;

#[doc(hidden)]
#[repr(C)]
pub struct UniqueData<'a, T: ?Sized> {
  free_vtable: FreeVtable<'a>,
  value: T,
}

impl Strategy for Unique {
  type SizedData<'allocator, T: 'allocator> = UniqueData<'allocator, T>;
  type SliceData<'allocator, T: SliceDst + ?Sized + 'allocator> = UniqueData<'allocator, T>;
  type Handle<'allocator, T: ?Sized + 'allocator> = UniqueHandle<'allocator, T>;

  fn initialize_data_sized<'allocator, T: 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut UniqueData<'allocator, T>,
  ) -> *mut T {
    unsafe {
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);

      (&raw mut (*data_ptr).value)
    }
  }

  fn initialize_data_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SliceData<'allocator, T>,
  ) -> *mut T {
    unsafe {
      (&raw mut (*data_ptr).free_vtable).write(free_vtable);

      (&raw mut (*data_ptr).value)
    }
  }

  fn construct_handle_sized<'allocator, T: 'allocator>(
    ptr: ptr::NonNull<UniqueData<'allocator, T>>,
  ) -> Self::Handle<'allocator, T> {
    UniqueHandle(ptr, Default::default())
  }

  fn construct_handle_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    ptr: ptr::NonNull<UniqueData<'allocator, T>>,
  ) -> Self::Handle<'allocator, T> {
    UniqueHandle(ptr, Default::default())
  }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct UniqueHandle<'allocator, T: ?Sized + 'allocator>(
  ptr::NonNull<UniqueData<'allocator, T>>,
  PhantomCovariantLifetime<'allocator>,
);

impl<'allocator, T: ?Sized> StrategyHandle<T> for UniqueHandle<'allocator, T> {
  type Cast<'cast, U: ?Sized + 'cast> = UniqueHandle<'cast, U>;

  fn as_ptr(&self) -> *mut T {
    unsafe { (&raw mut (*self.0.as_ptr()).value) }
  }

  fn into_strategy_data_ptr(self) -> StrategyDataPtr<T> {
    StrategyDataPtr {
      strategy_data: unsafe { ptr::NonNull::new_unchecked(self.0.as_ptr() as *mut _) },
      value: self.as_ptr(),
    }
  }

  unsafe fn from_strategy_data_ptr(
    StrategyDataPtr {
      strategy_data: start_ptr,
      value: value_ptr,
    }: StrategyDataPtr<T>,
  ) -> Self {
    let ptr = ptr::from_raw_parts_mut(start_ptr.as_ptr(), value_ptr.to_raw_parts().1);
    UniqueHandle(unsafe { ptr::NonNull::new_unchecked(ptr) }, Default::default())
  }
}

impl<'a, T: ?Sized> AsRef<T> for UniqueHandle<'a, T> {
  fn as_ref(&self) -> &T {
    unsafe { &self.0.as_ref().value }
  }
}

impl<'a, T: ?Sized> AsMut<T> for UniqueHandle<'a, T> {
  fn as_mut(&mut self) -> &mut T {
    unsafe { &mut self.0.as_mut().value }
  }
}

impl<'a, T: ?Sized> Deref for UniqueHandle<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    self.as_ref()
  }
}

impl<'a, T: ?Sized> DerefMut for UniqueHandle<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.as_mut()
  }
}

impl<'a, T: ?Sized> Drop for UniqueHandle<'a, T> {
  fn drop(&mut self) {
    unsafe { (&raw mut (*self.0.as_ptr()).free_vtable).read().free(self.0) };
  }
}

impl<'a, T: Debug + ?Sized> Debug for UniqueHandle<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.deref().fmt(f)
  }
}

impl<'a, T: Display + ?Sized> Display for UniqueHandle<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.deref().fmt(f)
  }
}

impl<'a, T: SliceDst + ?Sized> UninitStrategyHandleExt<UnsizedMaybeUninit<T>>
  for UniqueHandle<'a, UnsizedMaybeUninit<T>>
{
  type Init = UniqueHandle<'a, T>;

  unsafe fn assume_init(self) -> Self::Init {
    let StrategyDataPtr { strategy_data, value } = Self::into_strategy_data_ptr(self);
    let (ptr, size) = value.to_raw_parts();
    let value = ptr::from_raw_parts_mut(ptr, size);
    unsafe { UniqueHandle::from_strategy_data_ptr(StrategyDataPtr { strategy_data, value }) }
  }
}
