#![feature(derive_coerce_pointee)]
#![feature(phantom_variance_markers)]

use core::fmt::{Debug, Display};
use std::{
  cell::{Cell, UnsafeCell}, marker::{CoercePointee, PhantomInvariantLifetime}, ops::Deref, ptr::{NonNull, without_provenance_mut}
};

use aubystd::alloc::{Strategy, StrategyHandle, dst_allocator::DstAllocator};
use thiserror::Error;
use zerocopy::FromZeros;

#[repr(C)]
#[derive(Debug)]
struct A<T: Debug + ?Sized> {
  first: u32,
  second: u32,
  value: T,
}

impl<T: Debug + ?Sized> Display for A<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    self.value.fmt(f)
  }
}

struct TestStrategy;

impl Strategy for TestStrategy {
  type Data = ();
  type Handle<'a, T: ?Sized + 'a> = TestHandle<'a, T>;

  fn new_data() -> Self::Data {
    ()
  }

  fn construct_handle<'a, T: ?Sized>(ptr: NonNull<T>) -> Self::Handle<'a, T> {
    TestHandle(ptr, Default::default())
  }
}
#[derive(CoercePointee)]
#[repr(transparent)]
struct TestHandle<'a, T: ?Sized + 'a>(NonNull<T>, PhantomInvariantLifetime<'a>);
impl<'a, T: ?Sized> StrategyHandle<T> for TestHandle<'a, T> {
  fn as_ptr(&self) -> NonNull<T> {
    self.0
  }

  unsafe fn from_ptr(ptr: NonNull<T>) -> Self {
    TestHandle(ptr, Default::default())
  }
}
impl<'a, T: ?Sized> Deref for TestHandle<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { self.0.as_ref() }
  }
}

#[derive(Debug, Error)]
#[error("ran out of memory")]
struct OutOfMemory;
struct SimpleDynamicAllocator {
  storage: UnsafeCell<[u8; 4096]>,
  taken: Cell<bool>,
}

impl DstAllocator<TestStrategy> for SimpleDynamicAllocator {
  type AllocateError = OutOfMemory;

  async fn take_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<TestHandle<'allocator, T>, Self::AllocateError> {
    let offset = self.storage.get().align_offset(align_of::<T>());
    if self.taken.get() || offset + size_of::<T>() > 4096 {
      return Err(OutOfMemory);
    }
    self.taken.set(true);
    let offset = self.storage.get().align_offset(align_of::<T>());

    let ptr = unsafe {
      let ptr = self.storage.get().byte_add(offset);
      let ptr: *mut T = without_provenance_mut(ptr.addr());
      ptr.write(value);
      ptr
    };

    Ok(TestStrategy::construct_handle(NonNull::new(ptr).unwrap()))
  }

  async fn take_array<'allocator, T: 'allocator, const N: usize>(
    &'allocator self,
    value: [T; N],
  ) -> Result<<TestStrategy as Strategy>::Handle<'allocator, T>, Self::AllocateError> {
    let _ = value;
    todo!()
  }

  async fn take_from_iter<'allocator, T: 'allocator>(
    &'allocator self,
    iterator: impl ExactSizeIterator<Item = T>,
  ) -> Result<<TestStrategy as Strategy>::Handle<'allocator, [T]>, Self::AllocateError> {
    let _ = iterator;
    todo!()
  }

  async fn take_from_zeros<'allocator, T: FromZeros + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<<TestStrategy as Strategy>::Handle<'allocator, [T]>, Self::AllocateError> {
    let _ = element_count;
    todo!()
  }

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<<TestStrategy as Strategy>::Handle<'allocator, std::mem::MaybeUninit<T>>, Self::AllocateError> {
    let _ = value;
    todo!()
  }

  async fn reserve_array<'allocator, T: 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<<TestStrategy as Strategy>::Handle<'allocator, [std::mem::MaybeUninit<T>]>, Self::AllocateError> {
    let _ = element_count;
    todo!()
  }
}

#[pollster::main]
async fn main() {
  let allocator = Box::new(SimpleDynamicAllocator {
    storage: UnsafeCell::new_zeroed(),
    taken: Cell::new(false),
  });

  let handle: TestHandle<A<u32>> = allocator
    .take_item(A {
      first: 3,
      second: 4,
      value: 5,
    })
    .await
    .unwrap();
  println!("{}", &*handle);

  let handle: TestHandle<A<dyn Debug>> = handle;
  println!("{}", &*handle);

  drop(allocator);
}
