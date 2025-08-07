use core::fmt::Debug;

use thiserror::Error;

use crate::alloc::strategy::Arc;

pub unsafe trait Threading {
  async fn spawn<F: FnOnce(&dyn ThreadParker) + Send + Sync + 'static>(
    &'static self,
    stack_size: usize,
    func: F,
  ) -> Arc<'static, dyn ThreadHandle>;
}

pub trait ThreadParker {
  fn park(&self);
}

pub trait ThreadHandle {
  fn id(&self) -> usize;
  fn unpark(&self) -> Result<(), ThreadUnresponsive>;
}

#[aubystd_bikeshed_name("thread inactive")]
#[derive(Error, Debug)]
#[error("thread could not be woken: {reason}")]
pub struct ThreadUnresponsive {
  reason: &'static str,
}

pub struct ThreadId {}
