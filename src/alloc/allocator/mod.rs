pub mod arena;
pub mod foreign;

use core::{alloc::Layout, mem::MaybeUninit};

#[doc(inline)]
pub use arena::*;
#[doc(inline)]
pub use foreign::*;

use crate::alloc::strategy::Strategy;

pub trait Allocator<'s, T: 's> {
    type Error;

    async fn reserve<S: Strategy>(&'s self) -> Result<S::Handle<'s, MaybeUninit<T>>, Self::Error>;

    async fn take_item<S: Strategy>(&'s self, value: T) -> Result<S::Handle<'s, T>, Self::Error> {
        let handle = self.reserve::<S>().await?;
        unsafe { handle.as_ptr().cast::<T>().write(value) };
        Ok(unsafe { UninitStrategyHandleExt::assume_init(handle) })
    }
}

struct DstTemplate<T: ?Sized> {
    taken: bool,
    contents: T,
}

impl<T> DstTemplate<T> {
    fn contents(&self) -> Option<&T> {
        self.taken.then_some(&self.contents)
    }

    fn mark_taken(&mut self) {
        self.taken = true;
    }
}

pub trait UnsizedAllocator<'s, T: ?Sized + 's>: Allocator<'s, ()> {
    async fn reserve_unsized<S: Strategy>(
        &'s self,
        layout: Layout,
    ) -> Result<S::Handle<'s, [()]>, Self::Error>;

    async fn take_unsized<S: Strategy>(
        &'s self,
        source: &mut DstTemplate<T>,
    ) -> Result<S::Handle<'s, T>, Self::Error> {
        let handle = self.reserve_unsized::<S>()
    }
}

trait VecExt<T> {}

impl<T, S: StrategyHandle<Vec<T>>> VecExt<T> for S {}
