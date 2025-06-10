#![feature(more_qualified_paths)]
#![feature(custom_inner_attributes)]

#[allow(unused)]
use aubystd::prelude::*;

use aubystd::{
  alloc::{
    UnsizedMaybeUninit, allocator::{ArenaAllocator, ForeignAllocator, Malloc}, strategy::{Rc, Unique}
  }, zerocopy::FromZeros
};

use core::{
  cell::{Cell, RefCell, UnsafeCell}, fmt::{Debug, Display}, mem::MaybeUninit, pin::Pin, task::{Context, Poll}
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
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    struct Debugify<'a>(&'a [u32]);
    impl<'a> Debug for Debugify<'a> {
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

fn main() -> ! {
  pollster::block_on(async move {
    let allocator = ForeignAllocator::<Malloc>::default();

    let handle: Unique<[u32]> = allocator
      .take_from_iter(
        UNIQUE,
        core::iter::repeat_with(|| 5).enumerate().map(|(index, value)| value + index as u32).take(5),
      )
      .await
      .unwrap();

    println!("{:?}", handle);

    let mut handle: Unique<A<[u32]>> = allocator.take_from_zeros(UNIQUE, 5).await.unwrap();

    handle.value[2] = 5;

    println!("{}", handle);

    let handle = allocator.take_item(UNIQUE, [1, 2, 3, 4]).await.unwrap();

    println!("{:?}", handle);

    let handle: Unique<A<u32>> = allocator
      .take_item(
        UNIQUE,
        A {
          first: 3,
          second: 4,
          value: 5,
        },
      )
      .await
      .unwrap();
    println!("{}", handle);
    let handle: Unique<A<dyn Debug>> = handle;
    println!("{}", handle);

    let handle: Unique<A<[u8; 5]>> = allocator
      .take_item(
        UNIQUE,
        A {
          first: 3,
          second: 4,
          value: [9, 8, 7, 6, 5],
        },
      )
      .await
      .unwrap();
    println!("{}", handle);
    let handle: Unique<A<[u8]>> = handle;
    println!("{}", handle);

    let mut handle: Unique<UnsizedMaybeUninit<A<[u8]>>> = allocator.reserve_dst(UNIQUE, 5).await.unwrap();

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

    let mut handle: Unique<UnsizedMaybeUninit<A<D>>> = allocator.reserve_dst(UNIQUE, 5).await.unwrap();

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

    let handle: Unique<UnsafeCell<[MaybeUninit<u8>]>> =
      unsafe { allocator.reserve_dst(UNIQUE, 4096).await.unwrap().assume_init() };

    {
      let allocator = ArenaAllocator::new(handle);
      let _handle: Unique<[u8]> = allocator.take_from_zeros(UNIQUE, 1024).await.unwrap();
      let _handle: Unique<[u8]> = allocator.take_from_zeros(UNIQUE, 1024).await.unwrap();
      let _handle: Unique<[u8]> = allocator.take_from_zeros(UNIQUE, 1024).await.unwrap();
      _handle.into_pin();
      // unsafe { libc::printf(c"allocator remaining bytes: %d\n".as_ptr(), allocator.remaining()) }

      // let _handle: Unique<[u8]> = allocator.take_from_zeros(UNIQUE, 1024).await.unwrap();
    }

    #[derive(Default)]
    struct Tick(Cell<u32>);
    impl Future for Tick {
      type Output = ();

      fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.get() < 3 {
          _cx.waker().wake_by_ref();
          self.0.set(self.0.get() + 1);
          Poll::Pending
        } else {
          Poll::Ready(())
        }
      }
    }
    fn tick() -> impl Future {
      Tick::default()
    }
    let left: Rc<RefCell<u32>> = allocator.take_item(RC, 0.into()).await.unwrap();
    let right = left.clone();
    futures::future::join(
      async move {
        *left.borrow_mut() += 1;
        println!("left inc {}", left.borrow());
        tick().await;
        *left.borrow_mut() += 1;
        println!("left inc {}", left.borrow());
        tick().await;
        *left.borrow_mut() += 1;
        println!("left inc {}", left.borrow());
        tick().await;
      },
      async move {
        *right.borrow_mut() += 1;
        println!("right inc {}", right.borrow());
        tick().await;
        *right.borrow_mut() += 1;
        println!("right inc {}", right.borrow());
        tick().await;
        *right.borrow_mut() += 1;
        println!("right inc {}", right.borrow());
        tick().await;
      },
    )
    .await;

    let future = allocator
      .pin_item(UNIQUE, async move {
        println!("the beginning...");
        tick().await;
        println!("the end...");
        42
      })
      .await
      .unwrap();
    println!("the answer {}...", future.await);
  });

  unsafe { libc::exit(0) }
}
