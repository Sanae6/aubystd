#![feature(more_qualified_paths)]

use aubystd::{
  alloc::{
    Allocator, SliceAllocator, SliceDst, UnsizedMaybeUninit,
    allocator::{ArenaAllocator, ForeignAllocator, Malloc},
    slice_dst,
    strategy::{Rc, RcStrategy, Unique, UniqueStrategy},
  },
  prelude::UninitStrategyHandleExt,
  zerocopy::FromZeros,
};
use scratchpad::{block_on, println};

use core::{
  cell::{Cell, RefCell, UnsafeCell},
  fmt::{Debug, Display},
  mem::MaybeUninit,
  pin::Pin,
  task::{Context, Poll},
};

#[slice_dst(header = AHeader, derive(FromZeros))]
#[derive(FromZeros)]
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

#[slice_dst(header = BHeader)]
#[repr(C)]
struct B<A: Debug> {
  #[allow(unused)]
  first: u32,
  #[allow(unused)]
  second: u32,
  last: A,
}

#[slice_dst(header = CHeader)]
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

#[slice_dst(header = DHeader)]
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

async fn main_inner() {
  let allocator = ForeignAllocator::<Malloc>::default();

  // let handle: Unique<[u32]> = allocator
  //   .take_from_iter(
  //     UNIQUE,
  //     core::iter::repeat_with(|| 5).enumerate().map(|(index, value)| value + index as u32).take(5),
  //   )
  //   .await
  //   .unwrap();

  // println!("{:?}", handle);

  let mut handle: Unique<A<[u32]>> = allocator.from_zeros::<UniqueStrategy>(5).await.unwrap();

  handle.value[2] = 5;

  println!("{}", handle);

  let handle = allocator
    .take::<UniqueStrategy>([1, 2, 3, 4])
    .await
    .unwrap();

  println!("{:?}", handle);

  let handle: Unique<A<u32>> = allocator
    .take::<UniqueStrategy>(A {
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
    .take::<UniqueStrategy>(A {
      first: 3,
      second: 4,
      value: [9, 8, 7, 6, 5],
    })
    .await
    .unwrap();
  println!("{}", handle);
  let handle: Unique<A<[u8]>> = handle;
  println!("{}", handle);

  let mut handle: Unique<UnsizedMaybeUninit<A<[u8]>>> =
    allocator.reserve_slice::<UniqueStrategy>(5).await.unwrap();

  let handle = unsafe {
    handle.header.write(<A<[u8]> as SliceDst>::Header {
      first: 4,
      second: 5,
      value_header: (),
    });
    for index in 0..5 {
      handle.slice[index].write(index as u8 + 3);
    }
    Unique::assume_init(handle)
  };
  println!("{}", handle);
  let handle: Unique<A<[u8]>> = handle;
  println!("{}", handle);

  let mut handle: Unique<UnsizedMaybeUninit<A<D>>> =
    allocator.reserve_slice::<UniqueStrategy>(5).await.unwrap();

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
    Unique::assume_init(handle)
  };
  println!("{}", handle);

  let mut allocation_handle: Unique<UnsafeCell<[MaybeUninit<u8>]>> =
    allocator.from_zeros::<UniqueStrategy>(4096).await.unwrap();

  {
    let allocator = ArenaAllocator::new(&mut allocation_handle);
    let _handle1: Unique<[u8]> = allocator.from_zeros::<UniqueStrategy>(1024).await.unwrap();
    let _handle2: Unique<[u8]> = allocator.from_zeros::<UniqueStrategy>(1024).await.unwrap();
    let _handle3: Unique<[u8]> = allocator.from_zeros::<UniqueStrategy>(1024).await.unwrap();
    // unsafe { libc::printf(c"allocator remaining bytes: %d\n".as_ptr(), allocator.remaining()) }

    // let _handle: Unique<[u8]> = allocator.from_zeros(1024).await.unwrap();
    // drop((handle1, handle2, handle3));
    // drop(allocator);
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
  let left: Rc<RefCell<u32>> = allocator.take::<RcStrategy>(0.into()).await.unwrap();
  println!("value {}", left.borrow());
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
    .pin::<UniqueStrategy>(async move {
      println!("the beginning...");
      tick().await;
      println!("the end...");
      42
    })
    .await
    .unwrap();
  println!("the answer {}...", future.await);
}

fn main() {
  block_on(main_inner());
}
