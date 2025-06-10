#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(internal_features)]
#![allow(dead_code)]
#![allow(async_fn_in_trait)]
#![allow(refining_impl_trait)]
#![feature(derive_coerce_pointee, phantom_variance_markers, ptr_metadata)]
#![feature(more_maybe_bounds, trusted_len, prelude_import)]
#![feature(never_type, layout_for_ptr, deref_pure_trait)]
#![feature(tuple_trait, fn_traits, unsized_fn_params, unboxed_closures)]
#![feature(lang_items)]
#![cfg_attr(test, feature(assert_matches))]

#[macro_use]
#[allow(unused)]
pub extern crate core;
#[cfg(any(test, feature = "debugging"))]
#[macro_use]
#[allow(unused)]
pub extern crate std;

pub mod alloc;
pub use zerocopy;

pub mod prelude {
  pub use crate::alloc::{
    Allocator, ItemAllocator, SliceDst, strategy::{PinStrategyHandle, RC, StrategyHandle, UNIQUE, UninitStrategyHandleExt}
  };
  pub use aubystd_macros::epic;

  pub use core;
  #[cfg(not(any(test, feature = "debugging")))]
  pub use core::prelude::rust_2024::*;

  #[cfg(any(test, feature = "debugging"))]
  pub use std::{eprintln, prelude::rust_2024::*, println};
  pub use zerocopy;
}

#[prelude_import]
#[allow(unused)]
use prelude::*;
