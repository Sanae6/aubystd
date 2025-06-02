use core::{
  marker::{CoercePointee, PhantomCovariantLifetime}, ops::Deref, ptr::{NonNull, addr_of_mut}
};

use super::{Strategy, StrategyDataPtr, StrategyHandle};
use crate::alloc::{unsized_allocator::SliceDst, free::FreeVtable};

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
      addr_of_mut!((*data_ptr).free_vtable).write(free_vtable);

      addr_of_mut!((*data_ptr).value)
    }
  }

  fn initialize_data_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    free_vtable: FreeVtable<'allocator>,
    data_ptr: *mut Self::SliceData<'allocator, T>,
  ) -> *mut T {
    unsafe {
      addr_of_mut!((*data_ptr).free_vtable).write(free_vtable);

      addr_of_mut!((*data_ptr).value)
    }
  }

  fn construct_handle_sized<'allocator, T: 'allocator>(
    ptr: NonNull<UniqueData<'allocator, T>>,
  ) -> Self::Handle<'allocator, T> {
    UniqueHandle(ptr, Default::default())
  }

  fn construct_handle_slice<'allocator, T: SliceDst + ?Sized + 'allocator>(
    ptr: NonNull<UniqueData<'allocator, T>>,
  ) -> Self::Handle<'allocator, T> {
    UniqueHandle(ptr, Default::default())
  }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct UniqueHandle<'allocator, T: ?Sized + 'allocator>(
  NonNull<UniqueData<'allocator, T>>,
  PhantomCovariantLifetime<'allocator>,
);

impl<'allocator, T: ?Sized> StrategyHandle<T> for UniqueHandle<'allocator, T> {
  fn as_ptr(&self) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(addr_of_mut!((*self.0.as_ptr()).value)) }
  }

  fn into_strategy_data_ptr(self) -> StrategyDataPtr<T> {
    StrategyDataPtr(unsafe { NonNull::new_unchecked(self.0.as_ptr() as *mut _) })
  }

  unsafe fn from_strategy_data_ptr(StrategyDataPtr(ptr): StrategyDataPtr<T>) -> Self {
    UniqueHandle(
      unsafe { NonNull::new_unchecked(ptr.as_ptr() as *mut UniqueData<T>) },
      Default::default(),
    )
  }
}

impl<'a, T: ?Sized> Deref for UniqueHandle<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &self.0.as_ref().value }
  }
}

impl<'a, T: ?Sized> Drop for UniqueHandle<'a, T> {
  fn drop(&mut self) {
    unsafe { addr_of_mut!((*self.0.as_ptr()).free_vtable).read().free(self.0) };
  }
}
