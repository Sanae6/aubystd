use crate::alloc::{Identity, SliceAllocator, UnsizedMaybeUninit, strategy::Strategy};

pub trait StringAllocatorExt<'s>: SliceAllocator<'s, [u8]> {
  async fn take_string<S: Strategy>(&'s self, value: &str) -> Result<S::Handle<'s, str>, Self::Error>
  where
    S::Data<'s, UnsizedMaybeUninit<[u8]>>: SliceDst,
    <S::Handle<'s, UnsizedMaybeUninit<[u8]>> as StrategyHandle<'s, UnsizedMaybeUninit<[u8]>>>::Cast<str>:
      Identity<S::Handle<'s, str>>,
  {
    let handle = self.reserve_slice::<S>(value.len()).await?;

    Ok(unsafe { S::Handle::cast::<str>(handle).identity() })
  }
}

impl<'s, S: SliceAllocator<'s, [u8]>> StringAllocatorExt<'s> for S {}

pub trait StringExt {}

// impl<'a, S: Strategy> S::Handle< {

// }
