use core::{
  alloc::Layout,
  fmt::Debug,
  mem,
  ops::{Deref, DerefMut},
};

use crate::{
  alloc::{GrowthStrategy, SliceAllocator, UnsizedMaybeUninit, strategy::Strategy},
  types::vec::{BaseVecHeader, SliceVec},
};

#[repr(C)]
pub struct Vec<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, SliceVec<T>>>
where
  S::Handle<'a, SliceVec<T>>: Deref<Target = SliceVec<T>>,
{
  allocator: &'a A,
  growth_strategy: GrowthStrategy,
  inner: S::Handle<'a, SliceVec<T>>,
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, SliceVec<T>>> Vec<'a, T, S, A>
where
  S::Handle<'a, SliceVec<T>>: Deref<Target = SliceVec<T>>,
{
  pub async fn new(allocator: &'a A, strategy: GrowthStrategy) -> Result<Self, A::Error>
  where
    S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<T>>>:
      DerefMut<Target = UnsizedMaybeUninit<SliceVec<T>>>,
    S::Data<'a, UnsizedMaybeUninit<SliceVec<T>>>: SliceDst,
  {
    Self::with_capacity(allocator, strategy, 0).await
  }

  pub async fn with_capacity(
    allocator: &'a A,
    strategy: GrowthStrategy,
    capacity: usize,
  ) -> Result<Self, A::Error>
  where
    S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<T>>>:
      DerefMut<Target = UnsizedMaybeUninit<SliceVec<T>>>,
    S::Data<'a, UnsizedMaybeUninit<SliceVec<T>>>: SliceDst,
  {
    let mut handle: S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<T>>> =
      allocator.reserve_slice::<S>(capacity).await?;

    handle.header.write(BaseVecHeader::new());

    Ok(Self {
      allocator,
      growth_strategy: strategy,
      inner: unsafe { S::UninitHandle::assume_init(handle) },
    })
  }
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, SliceVec<T>>> Vec<'a, T, S, A>
where
  S::Handle<'a, SliceVec<T>>: DerefMut<Target = SliceVec<T>>,
  S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<T>>>:
    DerefMut<Target = UnsizedMaybeUninit<SliceVec<T>>>,
  S::Data<'a, UnsizedMaybeUninit<SliceVec<T>>>: SliceDst,
{
  pub async fn grow(&mut self, additional: usize) -> Result<(), A::Error> {
    let capacity = self
      .growth_strategy
      .calculate_new_capacity(self.inner.len(), additional)
      .expect("vec is full");

    self.resize(capacity).await
  }

  pub async fn resize(&mut self, to_capacity: usize) -> Result<(), A::Error> {
    let new: S::UninitHandle<'a, UnsizedMaybeUninit<SliceVec<T>>> =
      self.allocator.reserve_slice::<S>(to_capacity).await?;

    let old_ptr = S::Handle::as_value_ptr(&self.inner);
    let new_ptr = S::UninitHandle::as_value_ptr(&new);

    let new = unsafe {
      // Safety: same layout, non-overlapping ptrs
      old_ptr.cast::<u8>().copy_to_nonoverlapping(
        new_ptr.cast(),
        Layout::for_value_raw(old_ptr.cast_const()).size(),
      );
      // Safety: initialized by copying from self
      S::UninitHandle::assume_init(new)
    };

    mem::drop(mem::replace(&mut self.inner, new));

    Ok(())
  }

  pub async fn push_resize(&mut self, value: T) -> Result<(), A::Error> {
    if self.inner.capacity() < self.inner.len() + 1 {
      self.grow(1).await?;
    }

    let Ok(_) = self.inner.push(value) else {
      unreachable!("not enough space for value");
    };

    Ok(())
  }

  pub async fn push_resize_to(&mut self, value: T, to_capacity: usize) -> Result<(), A::Error> {
    if self.inner.capacity() < self.inner.len() + 1 {
      self.grow(to_capacity).await?;
    }

    let Ok(_) = self.inner.push(value) else {
      unreachable!("not enough space for value");
    };

    Ok(())
  }

  pub async fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) -> Result<(), A::Error> {
    let mut iter = iter.into_iter();

    while let Some(item) = iter.next() {
      let (lower_bound, _) = iter.size_hint();
      self.push_resize_to(item, lower_bound).await?;
    }

    Ok(())
  }
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, SliceVec<T>>> Deref for Vec<'a, T, S, A>
where
  S::Handle<'a, SliceVec<T>>: DerefMut<Target = SliceVec<T>>,
{
  type Target = SliceVec<T>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, SliceVec<T>>> DerefMut for Vec<'a, T, S, A>
where
  S::Handle<'a, SliceVec<T>>: DerefMut<Target = SliceVec<T>>,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<'a, T: Debug + 'a, S: Strategy, A: SliceAllocator<'a, SliceVec<T>>> Debug for Vec<'a, T, S, A>
where
  S::Handle<'a, SliceVec<T>>: DerefMut<Target = SliceVec<T>>,
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    self.deref().fmt(f)
  }
}
