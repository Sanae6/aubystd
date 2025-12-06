use core::task::{Context, Poll, Waker};

/// replacement for pollster for non-blocking futures.
/// genuinely useless if the waker is needed
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
  let mut future = core::pin::pin!(future);

  let mut cx = Context::from_waker(Waker::noop());
  loop {
    match future.as_mut().poll(&mut cx) {
      Poll::Ready(value) => return value,
      Poll::Pending => continue,
    }
  }
}
