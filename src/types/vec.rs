use core::{
  array, marker::PhantomData, mem::MaybeUninit, ops::{Deref, DerefMut, Index}
};

use aubystd_macros::slice_dst;

#[slice_dst(header = BaseVecHeader)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct BaseVec<T, V: AsRef<[MaybeUninit<T>]> + ?Sized> {
  phantom: PhantomData<T>,
  length: usize,
  values: V,
}

impl<T, V: AsRef<[MaybeUninit<T>]> + SliceDst + ?Sized> BaseVecHeader<T, V>
where
  V::Header: Default,
{
  pub fn new() -> Self {
    Self {
      length: 0,
      phantom: PhantomData,
      values_header: V::Header::default(),
    }
  }
}

pub type FixedVec<T, const CAPACITY: usize> = BaseVec<T, [MaybeUninit<T>; CAPACITY]>;

impl<T, const CAPACITY: usize> FixedVec<T, CAPACITY> {
  pub fn new() -> Self {
    Self {
      phantom: PhantomData,
      length: 0,
      values: array::from_fn(|_| MaybeUninit::uninit()),
    }
  }
}

impl<T, const CAPACITY: usize> Default for FixedVec<T, CAPACITY> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + ?Sized> Deref for BaseVec<T, V> {
  type Target = [T];

  fn deref(&self) -> &[T] {
    unsafe { std::mem::transmute(self.values.as_ref()) }
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> + ?Sized> DerefMut for BaseVec<T, V> {
  fn deref_mut(&mut self) -> &mut [T] {
    unsafe { std::mem::transmute(self.values.as_mut()) }
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + ?Sized> AsRef<[T]> for BaseVec<T, V> {
  fn as_ref(&self) -> &[T] {
    self
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> + ?Sized> AsMut<[T]> for BaseVec<T, V> {
  fn as_mut(&mut self) -> &mut [T] {
    self
  }
}
impl<T, V: AsRef<[MaybeUninit<T>]> + ?Sized> AsRef<Self> for BaseVec<T, V> {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> + ?Sized> AsMut<Self> for BaseVec<T, V> {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + ?Sized> BaseVec<T, V> {
  pub fn capacity(&self) -> usize {
    self.values.as_ref().len()
  }

  pub fn len(&self) -> usize {
    self.length
  }

  pub fn is_full(&self) -> bool {
    self.values.as_ref().len() <= self.length
  }

  pub fn is_empty(&self) -> bool {
    !self.is_full()
  }
}

impl<T, V: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> + ?Sized> BaseVec<T, V> {
  pub fn push(&mut self, value: T) -> Result<(), T> {
    if self.values.as_ref().len() < self.length + 1 {
      return Err(value);
    }

    self.values.as_mut()[self.length].write(value);
    self.length += 1;

    Ok(())
  }
}
