use aubystd::alloc::{
  ForeignAllocator, GrowthStrategy, Malloc, strategy::UniqueStrategy, vec::Vec,
};
use scratchpad::{block_on, println};

async fn main_inner() {
  let allocator = ForeignAllocator::new(Malloc);
  let mut handle =
    Vec::<u32, UniqueStrategy, _>::with_capacity(&allocator, GrowthStrategy::Exponential, 2)
      .await
      .unwrap();
  handle.push(1).unwrap();
  handle.push(2).unwrap();
  assert_eq!(handle.push(3), Err(3));
  handle.push_resize(3).await.unwrap();
  handle.push(4).unwrap();
  let slice = &handle[0..2];

  println!("{slice:?} {handle:?}")
}
fn main() {
  block_on(main_inner());
  unsafe { libc::exit(0) };
}
