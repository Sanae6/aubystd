use core::{
  fmt::Debug,
  ops::{Deref, DerefMut},
  str::Utf8Error,
};

use thiserror::Error;

use crate::{
  alloc::{
    GrowthStrategy, SliceAllocator, UnsizedMaybeUninit, strategy::Strategy, types::vec::Vec,
  },
  io::StreamWrite,
  types::vec::SliceVec,
};

pub struct String<'a, S: Strategy, A: SliceAllocator<'a, SliceVec<u8>>>
where
  S::Handle<'a, SliceVec<u8>>: Deref<Target = SliceVec<u8>>,
{
  inner: Vec<'a, u8, S, A>,
}

impl<'a, S: Strategy, A: SliceAllocator<'a, SliceVec<u8>>> String<'a, S, A>
where
  S::Handle<'a, SliceVec<u8>>: Deref<Target = SliceVec<u8>>,
{
  pub async fn new(allocator: &'a A, strategy: GrowthStrategy) -> Result<Self, A::Error>
  where
    S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<u8>>>:
      DerefMut<Target = UnsizedMaybeUninit<SliceVec<u8>>>,
    S::Data<'a, UnsizedMaybeUninit<SliceVec<u8>>>: SliceDst,
  {
    Ok(Self {
      inner: Vec::new(allocator, strategy).await?,
    })
  }

  pub async fn with_capacity(
    allocator: &'a A,
    strategy: GrowthStrategy,
    capacity: usize,
  ) -> Result<Self, A::Error>
  where
    S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<u8>>>:
      DerefMut<Target = UnsizedMaybeUninit<SliceVec<u8>>>,
    S::Data<'a, UnsizedMaybeUninit<SliceVec<u8>>>: SliceDst,
  {
    Ok(Self {
      inner: Vec::with_capacity(allocator, strategy, capacity).await?,
    })
  }
}

impl<'a, S: Strategy, A: SliceAllocator<'a, SliceVec<u8>>> String<'a, S, A>
where
  S::Handle<'a, SliceVec<u8>>: DerefMut<Target = SliceVec<u8>>,
  S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<u8>>>:
    DerefMut<Target = UnsizedMaybeUninit<SliceVec<u8>>>,
  S::Data<'a, UnsizedMaybeUninit<SliceVec<u8>>>: SliceDst,
{
  pub async fn push_str(&mut self, value: &str) -> Result<(), A::Error> {
    self.inner.extend(value.as_bytes().iter().cloned()).await?;

    Ok(())
  }

  pub async fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) -> Result<(), A::Error> {
    for item in iter {
      self.push_str(item).await?;
    }

    Ok(())
  }
}

#[derive(Error)]
#[error("{0}")]
pub enum WriteError<'a, A: SliceAllocator<'a, SliceVec<u8>>> {
  Allocator(A::Error),
  Utf8Error(#[from] Utf8Error),
}

impl<'a, A: SliceAllocator<'a, SliceVec<u8>>> Debug for WriteError<'a, A> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::Allocator(arg0) => f.debug_tuple("Allocator").field(arg0).finish(),
      Self::Utf8Error(arg0) => f.debug_tuple("Utf8Error").field(arg0).finish(),
    }
  }
}

impl<'a, S: Strategy, A: SliceAllocator<'a, SliceVec<u8>>> StreamWrite for String<'a, S, A>
where
  S::Handle<'a, SliceVec<u8>>: DerefMut<Target = SliceVec<u8>>,
  S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<u8>>>:
    DerefMut<Target = UnsizedMaybeUninit<SliceVec<u8>>>,
  S::Data<'a, UnsizedMaybeUninit<SliceVec<u8>>>: SliceDst,
{
  type Error = WriteError<'a, A>;
  async fn write<'b>(&mut self, data: &'b [u8]) -> Result<(usize, &'b [u8]), Self::Error> {
    let str = str::from_utf8(data)?;
    self.push_str(str).await.map_err(WriteError::Allocator)?;

    Ok((data.len(), &[]))
  }
}
