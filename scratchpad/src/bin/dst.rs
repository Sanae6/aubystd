#![feature(derive_coerce_pointee)]
#![feature(phantom_variance_markers)]
#![feature(array_ptr_get)]
#![feature(ptr_metadata)]
#![feature(slice_ptr_get)]
#![feature(trusted_len)]
#![feature(more_qualified_paths)]

use core::fmt::{Debug, Display};

use aubystd::{
  alloc::{
    Allocator, SliceDst, UnsizedMaybeUninit, allocators::{ForeignAllocator, StdAlloc}, strategy::{UninitStrategyHandleExt, Unique, UniqueStrategy}
  }, zerocopy::FromZeros
};

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
#[repr(C)]
struct B<A: Debug> {
  #[allow(unused)]
  first: u32,
  #[allow(unused)]
  second: u32,
  last: A,
}

#[derive(SliceDst)]
#[repr(C)]
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
#[repr(C)]
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

    f.debug_struct("D")
      .field("first", &self.first)
      .field("second", &self.second)
      .field("last", &Debugify(&self.last))
      .finish()
  }
}

fn main() {
  pollster::block_on(async move {
    let allocator = ForeignAllocator::<StdAlloc, UniqueStrategy>::default();

    let handle: Unique<[u32]> = allocator
      .take_from_iter(core::iter::repeat_with(|| 5).enumerate().map(|(index, value)| value + index as u32).take(5))
      .await
      .unwrap();

    for ele in &*handle {
      println!("{}", ele);
    }

    let mut handle: Unique<A<[u32]>> = allocator.take_from_zeros(5).await.unwrap();

    handle.value[2] = 5;

    println!("{}", handle);

    let handle: Unique<[u32]> = allocator.take_item([1, 2, 3, 4]).await.unwrap();

    println!("{:?}", handle);

    let handle: Unique<A<u32>> = allocator
      .take_item(A {
        first: 3,
        second: 4,
        value: 5,
      })
      .await
      .unwrap();
    println!("{}", handle);
    let handle: Unique<A<dyn Debug>> = handle;
    println!("{}", handle);

    let handle: Unique<A<[u8; 5]>> = allocator
      .take_item(A {
        first: 3,
        second: 4,
        value: [9, 8, 7, 6, 5],
      })
      .await
      .unwrap();
    println!("{}", handle);
    let handle: Unique<A<[u8]>> = handle;
    println!("{}", handle);

    println!();
    println!();
    println!();

    let mut handle: Unique<UnsizedMaybeUninit<A<[u8]>>> = allocator.reserve_dst(5).await.unwrap();

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
    let handle: Unique<A<[u8]>> = handle;
    println!("{}", handle);

    let mut handle: Unique<UnsizedMaybeUninit<A<D>>> = allocator.reserve_dst(5).await.unwrap();

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
