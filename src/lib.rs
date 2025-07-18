#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(internal_features)]
#![allow(dead_code)]
#![allow(async_fn_in_trait)]
#![allow(refining_impl_trait)]
#![feature(derive_coerce_pointee, phantom_variance_markers, ptr_metadata)]
#![feature(more_maybe_bounds, trusted_len, prelude_import)]
#![feature(never_type, layout_for_ptr, deref_pure_trait, sync_unsafe_cell)]
#![feature(lang_items)]
#![cfg_attr(test, feature(assert_matches))]

#[macro_use]
#[allow(unused)]
pub extern crate core;

pub mod alloc;
pub mod futures;
pub mod io;
pub mod num;
pub mod platform;
mod rt;
pub mod thread;
pub mod types;

#[cfg(feature = "libc")]
pub use libc;
pub use zerocopy;

pub mod prelude {
  pub use crate::alloc::{
    Allocator, SliceDst, strategy::{PinStrategyHandle, RC, Rc, StrategyHandle, UNIQUE, UninitStrategyHandleExt, Unique}
  };
  #[doc(hidden)]
  pub(crate) use aubystd_macros::aubystd_bikeshed_name;
  pub use core::prelude::rust_2024::*;

  pub use zerocopy;
}
#[cfg(test)]
#[cfg(feature = "libc")]
pub use crate::alloc::arena::test_arena;

#[prelude_import]
#[allow(unused)]
use prelude::*;
