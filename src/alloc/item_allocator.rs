use core::mem::MaybeUninit;

use super::Strategy;

pub trait ItemAllocator<T, S: Strategy> {
  type AllocateError;

  async fn take<'allocator>(&'allocator self, value: T) -> Result<S::Handle<'allocator, T>, Self::AllocateError>
  where
    T: 'allocator;

  async fn reserve<'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, MaybeUninit<T>>, Self::AllocateError>
  where
    T: 'allocator;
}

#[cfg(not(true))]
mod testing {
  use core::{
    cell::{RefCell, UnsafeCell}, ops::{Deref, DerefMut}
  };

  use thiserror::Error;
  use zerocopy::FromZeros;

  use super::ItemAllocator;

  #[derive(Debug, Error)]
  #[error("ran out of memory!")]
  struct OutOfMemory;
  struct SingleItemAllocator(RefCell<Option<&'static UnsafeCell<i32>>>);
  impl ItemAllocator<i32> for SingleItemAllocator {
    type AllocateError = OutOfMemory;
    type Box<'a> = ExampleHandle<'a>;
    fn allocate<'allocator>(&'allocator self) -> Result<Self::Box<'allocator>, Self::AllocateError>
    where
      i32: FromZeros,
    {
      self.0.borrow_mut().take().map(|cell| ExampleHandle(self, cell)).ok_or(OutOfMemory)
    }
  }

  struct ExampleHandle<'a>(&'a SingleItemAllocator, &'static UnsafeCell<i32>);
  impl<'a> Box<i32> for ExampleHandle<'a> {}
  impl<'a> Deref for ExampleHandle<'a> {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
      unsafe { &*self.1.get() }
    }
  }
  impl<'a> DerefMut for ExampleHandle<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      unsafe { &mut *self.1.get() }
    }
  }
  impl<'a> Drop for ExampleHandle<'a> {
    fn drop(&mut self) {
      assert!(self.0.0.borrow_mut().replace(self.1).is_none());
    }
  }
  #[pollster::test]
  pub async fn test() {
    let allocator = SingleItemAllocator({
      // impl details
      static mut ALLOCATION: UnsafeCell<i32> = UnsafeCell::new(0);
      RefCell::new(Some(unsafe { &mut *&raw mut ALLOCATION }))
    });

    let handle = allocator.allocate().expect("first allocation failed");
    assert!(allocator.allocate().is_err());
    drop(handle);
    drop(allocator.allocate().expect("second allocation failed"));
    allocator.allocate().expect("third allocation failed");
  }
}
