#![feature(derive_coerce_pointee)]
#![feature(phantom_variance_markers)]
#![feature(array_ptr_get)]
#![feature(ptr_metadata)]
#![feature(slice_ptr_get)]

use core::fmt::{Debug, Display};
use std::{
  alloc::Layout, cell::{Cell, UnsafeCell}, marker::PhantomData, mem::MaybeUninit, ptr::{addr_of_mut, from_raw_parts, from_raw_parts_mut, NonNull}
};

use aubystd::alloc::{
  free::FreeVtable, strategy::{
    Strategy, unique::{Unique, UniqueHandle}
  }, unsized_allocator::{Allocator, SliceDst}
};
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

#[repr(C)]
#[derive(FromZeros)]
struct Header {
  first: u32,
  second: u32,
}
impl<T: Debug> SliceDst for A<[T]> {
  type Header = Header;

  type Element = T;

  fn addr_of_elements(ptr: *mut Self) -> *mut [Self::Element] {
    unsafe { addr_of_mut!((*ptr).value) }
  }
}

#[derive(Debug, Error)]
#[error("ran out of memory")]
struct OutOfMemory;
struct SimpleDynamicAllocator<S: Strategy> {
  storage: UnsafeCell<[u8; 4096]>,
  taken: Cell<bool>,
  strategy: PhantomData<S>,
}

impl<S: Strategy> SimpleDynamicAllocator<S> {
  fn create_free_vtable<'allocator>(&'allocator self) -> FreeVtable<'allocator> {
    FreeVtable::new(Self::free, &raw const *self as *mut Self)
  }

  /// Safety: The context provided to the free function must be a pointer to the allocator.
  unsafe fn free(context: *mut (), allocation: *mut ()) {
    // Safety: See above. The context is never accessed mutably, so we can freely get an immutable reference.
    let this = unsafe { context.cast::<Self>().as_ref().expect("null context was provided") };

    // Safety: When freeing, we have full control over the data in the storage.
    // As such, it is safe to create an immutable reference.
    assert!(unsafe {
      this.storage.get().as_ref().unwrap().as_ptr_range().contains(&(allocation as *mut _ as *const _))
    });
    this.taken.set(false);
  }
}

impl<S: Strategy> Allocator<S> for SimpleDynamicAllocator<S> {
  type AllocateError = OutOfMemory;

  async fn take_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError> {
    let offset = self.storage.get().as_mut_ptr().align_offset(align_of::<S::SizedData<'allocator, T>>());
    if self.taken.get()
      || offset == usize::MAX
      || offset.saturating_add(size_of::<S::SizedData<'allocator, T>>()) > 4096
    {
      return Err(OutOfMemory);
    }

    self.taken.set(true);

    let data_ptr = unsafe {
      let data_ptr = self.storage.get().byte_add(offset);
      let data_ptr: *mut S::SizedData<'allocator, T> = data_ptr as *mut _;
      let value_ptr = S::initialize_data_sized(self.create_free_vtable(), data_ptr);
      value_ptr.write(value);
      data_ptr
    };

    Ok(S::construct_handle_sized(NonNull::new(data_ptr).unwrap()))
  }

  async fn take_array<'allocator, T: 'allocator, const N: usize>(
    &'allocator self,
    value: [T; N],
  ) -> Result<S::Handle<'allocator, [T; N]>, Self::AllocateError> {
    self.take_item(value).await
  }

  async fn take_from_iter<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    iterator: impl ExactSizeIterator<Item = T::Element>,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError> {
    let max_length = iterator.len();
    let layout = Layout::new::<S::SizedData<'allocator, T::Header>>();
    let array = Layout::array::<T::Element>(max_length).map_err(|_| OutOfMemory)?;
    let layout = layout.extend(array).map_err(|_| OutOfMemory)?.0;
    let offset = self.storage.get().as_mut_ptr().align_offset(layout.align());
    if self.taken.get() || offset == usize::MAX || offset.saturating_add(layout.size()) > 4096 {
      return Err(OutOfMemory);
    }

    self.taken.set(true);

    let data_ptr = unsafe {
      let data_ptr = self.storage.get().byte_add(offset);
      let data_ptr: *mut S::SliceData<'allocator, T> = from_raw_parts_mut(data_ptr, max_length);
      let value_ptr = S::initialize_data_slice(self.create_free_vtable(), data_ptr);
      value_ptr.cast::<T::Header>().write_bytes(0, 1);
      let slice_ptr = T::addr_of_elements(value_ptr);
      let mut actual_length = 0;
      for (index, value) in iterator.enumerate() {
        slice_ptr.get_unchecked_mut(index).write(value);
        actual_length = index;
      }

      assert!(actual_length <= max_length);

      let data_ptr = from_raw_parts_mut(data_ptr.to_raw_parts().0, actual_length);

      data_ptr
    };

    Ok(S::construct_handle_slice(NonNull::new(data_ptr).unwrap()))
  }

  async fn take_from_zeros<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError>
  where
    T::Element: FromZeros,
  {
    let _ = element_count;
    todo!()
  }

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
    value: T,
  ) -> Result<S::Handle<'allocator, MaybeUninit<T>>, Self::AllocateError> {
    let _ = value;
    todo!()
  }


  async fn reserve_dst<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<<S as Strategy>::Handle<'allocator, T>, Self::AllocateError> {
    let _ = element_count;
    todo!()
  }
}

#[pollster::main]
async fn main() {
  let allocator = Box::new(SimpleDynamicAllocator::<Unique> {
    storage: UnsafeCell::new_zeroed(),
    taken: Cell::new(false),
    strategy: PhantomData,
  });

  // let handle: UniqueHandle<A<[u32]>> = allocator.take_from_iter(core::iter::repeat_with(|| 5).take(5)).await.unwrap();

  // println!("{}", handle)

  let handle: UniqueHandle<A<u32>> = allocator
    .take_item(A {
      first: 3,
      second: 4,
      value: 5,
    })
    .await
    .unwrap();
  println!("{}", &*handle);
  let handle: UniqueHandle<A<dyn Debug>> = handle;
  println!("{}", &*handle);
  drop(handle);

  let handle: UniqueHandle<A<[u8; 5]>> = allocator
    .take_item(A {
      first: 3,
      second: 4,
      value: [9, 8, 7, 6, 5],
    })
    .await
    .unwrap();
  println!("{}", &*handle);
  let handle: UniqueHandle<A<[u8]>> = handle;
  println!("{}", &*handle);

  // drop(allocator);
}
