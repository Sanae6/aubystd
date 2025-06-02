#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(internal_features)]
#![allow(dead_code)]
#![allow(async_fn_in_trait)]
#![feature(derive_coerce_pointee)]
#![feature(phantom_variance_markers)]
#![feature(ptr_metadata)]
#![feature(more_maybe_bounds)]

pub mod alloc;
pub use zerocopy;
