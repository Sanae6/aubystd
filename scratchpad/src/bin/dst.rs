#![feature(derive_coerce_pointee)]
#![feature(phantom_variance_markers)]
#![feature(array_ptr_get)]
#![feature(ptr_metadata)]
#![feature(slice_ptr_get)]
#![feature(trusted_len)]
#![feature(more_qualified_paths)]

use core::fmt::{Debug, Display};
use std::{
  alloc::Layout, cell::{Cell, UnsafeCell}, iter::TrustedLen, marker::PhantomData, mem::MaybeUninit, ptr::{NonNull, from_raw_parts_mut}
};

use aubystd::{
  alloc::{
    Allocator, FreeVtable, SliceDst, Strategy, UninitStrategyHandleExt, UnsizedMaybeUninit, unique::{Unique, UniqueHandle}
  }, zerocopy::FromZeros
};
use thiserror::Error;

#[derive(SliceDst, FromZeros)]
#[repr(C)]
#[zerocopy(crate = "aubystd::zerocopy")]
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

#[derive(SliceDst)]
struct B<A: Debug> {
  #[allow(unused)]
  first: u32,
  #[allow(unused)]
  second: u32,
  last: A,
}

#[derive(SliceDst)]
struct C<A: Debug> {
  #[allow(unused)]
  first: u32,
  #[allow(unused)]
  second: u32,
  #[allow(unused)]
  third: A,
  last: [u32],
}

#[derive(SliceDst)]
struct D {
  #[allow(unused)]
  first: u32,
  #[allow(unused)]
  second: u32,
  last: [u32],
}

impl Debug for D {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    struct Debugify<'a>(&'a [u32]);
    impl<'a> Debug for Debugify<'a> {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
      }
    }
     
    f.debug_struct("D").field("first", &self.first).field("second", &self.second).field("last", &Debugify(&self.last)).finish()
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
    iterator: impl TrustedLen<Item = T::Element>,
  ) -> Result<S::Handle<'allocator, T>, Self::AllocateError> {
    let (_, Some(max_length)) = iterator.size_hint() else {
      return Err(OutOfMemory);
    };

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
      let slice_ptr = T::addr_of_slice(value_ptr);
      let mut actual_length = 0;
      for (index, value) in iterator.enumerate() {
        slice_ptr.get_unchecked_mut(index).write(value);
        actual_length = index + 1;
      }

      assert_eq!(actual_length, max_length);

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
    let layout = Layout::new::<S::SizedData<'allocator, T::Header>>();
    let array = Layout::array::<T::Element>(element_count).map_err(|_| OutOfMemory)?;
    let layout = layout.extend(array).map_err(|_| OutOfMemory)?.0;

    let offset = self.storage.get().as_mut_ptr().align_offset(layout.align());

    if self.taken.get() || offset == usize::MAX || offset.saturating_add(layout.size()) > 4096 {
      return Err(OutOfMemory);
    }

    self.taken.set(true);

    let data_ptr = unsafe {
      let data_ptr = self.storage.get().byte_add(offset);
      let data_ptr: *mut S::SliceData<'allocator, T> = from_raw_parts_mut(data_ptr, element_count);

      let value_ptr = S::initialize_data_slice(self.create_free_vtable(), data_ptr);
      value_ptr.cast::<T::Header>().write_bytes(0, 1);
      let slice_ptr = T::addr_of_slice(value_ptr);
      core::ptr::write_bytes(slice_ptr.as_mut_ptr() as *mut u8, 0, array.size());

      data_ptr
    };

    Ok(S::construct_handle_slice(NonNull::new(data_ptr).unwrap()))
  }

  async fn reserve_item<'allocator, T: 'allocator>(
    &'allocator self,
  ) -> Result<S::Handle<'allocator, MaybeUninit<T>>, Self::AllocateError> {
    let offset = self.storage.get().as_mut_ptr().align_offset(align_of::<S::SizedData<'allocator, MaybeUninit<T>>>());
    if self.taken.get()
      || offset == usize::MAX
      || offset.saturating_add(size_of::<S::SizedData<'allocator, MaybeUninit<T>>>()) > 4096
    {
      return Err(OutOfMemory);
    }

    self.taken.set(true);

    let data_ptr = unsafe {
      let data_ptr = self.storage.get().byte_add(offset);
      let data_ptr: *mut S::SizedData<'allocator, MaybeUninit<T>> = data_ptr as *mut _;

      let _value_ptr = S::initialize_data_sized(self.create_free_vtable(), data_ptr);
      data_ptr
    };

    Ok(S::construct_handle_sized(NonNull::new(data_ptr).unwrap()))
  }

  async fn reserve_dst<'allocator, T: SliceDst + ?Sized + 'allocator>(
    &'allocator self,
    element_count: usize,
  ) -> Result<<S as Strategy>::Handle<'allocator, UnsizedMaybeUninit<T>>, Self::AllocateError> {
    let layout = Layout::new::<S::SizedData<'allocator, T::Header>>();
    let array = Layout::array::<T::Element>(element_count).map_err(|_| OutOfMemory)?;
    let layout = layout.extend(array).map_err(|_| OutOfMemory)?.0;

    let offset = self.storage.get().as_mut_ptr().align_offset(layout.align());

    if self.taken.get() || offset == usize::MAX || offset.saturating_add(layout.size()) > 4096 {
      return Err(OutOfMemory);
    }

    self.taken.set(true);

    let data_ptr = unsafe {
      let data_ptr = self.storage.get().byte_add(offset);
      let data_ptr: *mut S::SliceData<'allocator, UnsizedMaybeUninit<T>> = from_raw_parts_mut(data_ptr, element_count);

      let _value_ptr = S::initialize_data_slice::<UnsizedMaybeUninit<T>>(self.create_free_vtable(), data_ptr);
      data_ptr
    };

    Ok(S::construct_handle_slice(NonNull::new(data_ptr).unwrap()))
  }
}

fn main() {
  pollster::block_on(async move {
    let allocator = Box::new(SimpleDynamicAllocator::<Unique> {
      storage: UnsafeCell::new_zeroed(),
      taken: Cell::new(false),
      strategy: PhantomData,
    });

    let handle: UniqueHandle<A<[u32]>> = allocator
      .take_from_iter(core::iter::repeat_with(|| 5).enumerate().map(|(index, value)| value + index as u32).take(5))
      .await
      .unwrap();

    println!("{}", handle);
    drop(handle);

    let handle: UniqueHandle<A<[u32]>> = allocator.take_from_zeros(5).await.unwrap();

    println!("{}", handle);
    drop(handle);

    let handle: UniqueHandle<[u32]> = allocator.take_array([1, 2, 3, 4]).await.unwrap();

    println!("{:?}", handle);
    drop(handle);

    let handle: UniqueHandle<A<u32>> = allocator
      .take_item(A {
        first: 3,
        second: 4,
        value: 5,
      })
      .await
      .unwrap();
    println!("{}", handle);
    let handle: UniqueHandle<A<dyn Debug>> = handle;
    println!("{}", handle);
    drop(handle);

    let handle: UniqueHandle<A<[u8; 5]>> = allocator
      .take_item(A {
        first: 3,
        second: 4,
        value: [9, 8, 7, 6, 5],
      })
      .await
      .unwrap();
    println!("{}", handle);
    let handle: UniqueHandle<A<[u8]>> = handle;
    println!("{}", handle);

    drop(handle);

    println!();
    println!();
    println!();

    let mut handle: UniqueHandle<UnsizedMaybeUninit<A<[u8]>>> = allocator.reserve_dst(5).await.unwrap();

    let handle = unsafe {
      handle.header.write(<A<[u8]> as SliceDst>::Header {
        first: 4,
        second: 5,
        value_header: (),
      });
      for index in 0..5 {
        handle.slice[index].write(index as u8 + 3);
      }
      handle.assume_init()
    };
    println!("{}", handle);
    let handle: UniqueHandle<A<[u8]>> = handle;
    println!("{}", handle);
    drop(handle);

    let mut handle: UniqueHandle<UnsizedMaybeUninit<A<D>>> = allocator.reserve_dst(5).await.unwrap();

    let handle = unsafe {
      handle.header.write(<A<D> as SliceDst>::Header {
        first: 4,
        second: 5,
        value_header: <D as SliceDst>::Header {
          first: 0,
          second: 0,
          last_header: (),
        },
      });
      for index in 0..5 {
        handle.slice[index].write(index as u32 + 3);
      }
      handle.assume_init()
    };
    println!("{}", handle);
  })
}
