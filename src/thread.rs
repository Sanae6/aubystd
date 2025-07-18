use core::fmt::Debug;

use thiserror::Error;

pub trait Threading {
  fn spawn<'a>(&'a self, stack_size: u32, func: impl FnOnce() + Send + Sync + 'a) -> ThreadHandle<'a>;
}
pub trait ThreadHandleInner {
  fn id(&self) -> usize;
  fn wake(&self) -> Result<(), ThreadUnresponsive>;
}

#[aubystd_bikeshed_name("thread inactive")]
#[derive(Error, Debug)]
#[error("thread could not be woken: {reason}")]
pub struct ThreadUnresponsive {
  reason: &'static str,
}

pub struct ThreadHandle<'a>(Unique<'a, dyn ThreadHandleInner>);

pub struct ThreadId {}
