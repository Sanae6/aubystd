use aubystd::alloc::{strategy::UniqueStrategy, vec::{GrowthStrategy, Vec}, ForeignAllocator, Malloc};
#[allow(unused)]
use aubystd::prelude::*;

async fn main_inner() {
  let allocator = ForeignAllocator::new(Malloc);
  let mut handle = Vec::<u32, UniqueStrategy, _>::with_capacity(&allocator, GrowthStrategy::Exponential, 2).await.unwrap();
  handle.push(1).unwrap();
  handle.push(2).unwrap();
  assert_eq!(handle.push(3), Err(3));
  handle.push_resize(3).await.unwrap();
  handle.push(4).unwrap();

  println!("{handle:?}")
}
fn main() {
  pollster::block_on(main_inner());
  unsafe { libc::exit(0) };
}
