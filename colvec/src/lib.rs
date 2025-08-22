#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

pub use colvec_derive::ColVec;

mod error;

// used from generated code

#[doc(hidden)]
pub mod alloc;
#[doc(hidden)]
pub mod fields;
#[doc(hidden)]
pub mod raw;
