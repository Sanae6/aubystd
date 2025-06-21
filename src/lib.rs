#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(internal_features)]
#![allow(dead_code)]
#![allow(async_fn_in_trait)]
#![allow(refining_impl_trait)]
#![feature(derive_coerce_pointee, phantom_variance_markers, ptr_metadata)]
#![feature(more_maybe_bounds, trusted_len, prelude_import)]
#![feature(never_type, layout_for_ptr, deref_pure_trait, sync_unsafe_cell)]
#![feature(tuple_trait, fn_traits, unsized_fn_params, unboxed_closures)]
#![cfg_attr(test, feature(assert_matches))]

#[macro_use]
#[allow(unused)]
pub extern crate core;
#[cfg(any(test, feature = "debugging"))]
#[macro_use]
#[allow(unused)]
pub extern crate std;

pub mod alloc;
pub mod io;
pub mod num;
pub use zerocopy;

pub mod prelude {
  pub use crate::alloc::{
    allocator::Allocator, ItemAllocator, SliceDst, strategy::{PinStrategyHandle, RC, Rc, StrategyHandle, UNIQUE, UninitStrategyHandleExt, Unique}
  };
  pub use aubystd_macros::epic;

  pub use core::prelude::rust_2024::*;
  #[cfg(any(test, feature = "debugging"))]
  pub use std::{eprintln, prelude::rust_2024::*, println};
  pub use zerocopy;
}

#[prelude_import]
#[allow(unused)]
use prelude::*;
