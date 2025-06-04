#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(internal_features)]
#![allow(dead_code)]
#![allow(async_fn_in_trait)]
#![feature(derive_coerce_pointee)]
#![feature(phantom_variance_markers)]
#![feature(ptr_metadata)]
#![feature(more_maybe_bounds)]
#![feature(trusted_len)]
#![feature(prelude_import)]

pub mod alloc;
pub use zerocopy;

pub mod prelude {
  pub use zerocopy;
  pub use core::prelude::rust_2024::*;
}

#[prelude_import]
#[allow(unused)]
use prelude::*;
