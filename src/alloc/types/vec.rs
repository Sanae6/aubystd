use core::{
  alloc::Layout, cmp, fmt::Debug, mem::{self, MaybeUninit}, ops::{Deref, DerefMut}
};

use crate::{
  alloc::{SliceAllocator, UnsizedMaybeUninit, strategy::Strategy}, types::vec::{BaseVec, BaseVecHeader}
};

pub enum GrowthStrategy {
  /// Grow the capacity of the [Vec] by exactly the amount that is needed
  Exact,
  /// Grow the internal of the [Vec] by 2 * the previous capacity, or the exact capacity that is required, whichever is larger.
  Exponential,
}

impl GrowthStrategy {
  pub fn calculate_new_capacity(&self, capacity: usize, additional: usize) -> Option<usize> {
    let min_capacity = capacity.checked_add(additional)?;
    match self {
      GrowthStrategy::Exact => Some(min_capacity),
      GrowthStrategy::Exponential => Some(cmp::max(capacity.checked_mul(2)?, min_capacity)),
    }
  }
}

type InnerVec<T> = BaseVec<T, [MaybeUninit<T>]>;

#[repr(C)]
pub struct Vec<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, InnerVec<T>>>
where
  S::Handle<'a, InnerVec<T>>: Deref<Target = InnerVec<T>>,
{
  allocator: &'a A,
  growth_strategy: GrowthStrategy,
  inner: S::Handle<'a, InnerVec<T>>,
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, InnerVec<T>>> Vec<'a, T, S, A>
where
  S::Handle<'a, InnerVec<T>>: Deref<Target = InnerVec<T>>,
{
  pub async fn new(allocator: &'a A, strategy: GrowthStrategy) -> Result<Self, A::Error>
  where
    S::UninitHandle<'a, UnsizedMaybeUninit<InnerVec<T>>>: DerefMut<Target = UnsizedMaybeUninit<InnerVec<T>>>,
    S::Data<'a, UnsizedMaybeUninit<InnerVec<T>>>: SliceDst,
  {
    Self::with_capacity(allocator, strategy, 0).await
  }

  pub async fn with_capacity(allocator: &'a A, strategy: GrowthStrategy, capacity: usize) -> Result<Self, A::Error>
  where
    S::UninitHandle<'a, UnsizedMaybeUninit<InnerVec<T>>>: DerefMut<Target = UnsizedMaybeUninit<InnerVec<T>>>,
    S::Data<'a, UnsizedMaybeUninit<InnerVec<T>>>: SliceDst,
  {
    let mut handle: S::UninitHandle<'a, UnsizedMaybeUninit<InnerVec<T>>> =
      allocator.reserve_slice::<S>(capacity).await?;

    handle.header.write(BaseVecHeader::new());

    Ok(Self {
      allocator,
      growth_strategy: strategy,
      inner: unsafe { S::UninitHandle::assume_init(handle) },
    })
  }
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, InnerVec<T>>> Vec<'a, T, S, A>
where
  S::Handle<'a, InnerVec<T>>: DerefMut<Target = InnerVec<T>>,
  S::UninitHandle<'a, UnsizedMaybeUninit<InnerVec<T>>>: DerefMut<Target = UnsizedMaybeUninit<InnerVec<T>>>,
  S::Data<'a, UnsizedMaybeUninit<InnerVec<T>>>: SliceDst,
{
  pub async fn grow(&mut self, additional: usize) -> Result<(), A::Error> {
    let capacity = self.growth_strategy.calculate_new_capacity(self.inner.len(), additional).expect("vec is full");

    self.resize(capacity).await
  }

  pub async fn resize(&mut self, to_capacity: usize) -> Result<(), A::Error> {
    let new: S::UninitHandle<'a, UnsizedMaybeUninit<InnerVec<T>>> =
      self.allocator.reserve_slice::<S>(to_capacity).await?;

    unsafe {
      let old = S::Handle::as_value_ptr(&self.inner);
      let new = S::UninitHandle::as_value_ptr(&new);

      old.cast::<u8>().copy_to_nonoverlapping(new.cast(), Layout::for_value_raw(old.cast_const()).size());
    };

    mem::drop(mem::replace(&mut self.inner, unsafe {
      S::UninitHandle::assume_init(new)
    }));

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

  pub fn push(&mut self, value: T) -> Result<(), T> {
    self.inner.push(value)
  }
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, InnerVec<T>>> Deref for Vec<'a, T, S, A>
where
  S::Handle<'a, InnerVec<T>>: DerefMut<Target = InnerVec<T>>,
{
  type Target = [T];

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<'a, T: 'a, S: Strategy, A: SliceAllocator<'a, InnerVec<T>>> DerefMut for Vec<'a, T, S, A>
where
  S::Handle<'a, InnerVec<T>>: DerefMut<Target = InnerVec<T>>,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<'a, T: Debug + 'a, S: Strategy, A: SliceAllocator<'a, InnerVec<T>>> Debug for Vec<'a, T, S, A>
where
  S::Handle<'a, InnerVec<T>>: DerefMut<Target = InnerVec<T>>,
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    self.deref().fmt(f)
  }
}
