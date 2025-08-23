#[cfg(feature = "allocator-api2")]
pub use allocator_api2::alloc::*;
#[cfg(feature = "nightly")]
pub use core::alloc::*;

#[cfg(all(feature = "allocator-api2",feature = "std"))]
pub use allocator_api2::alloc::Global;
#[cfg(all(feature = "nightly",feature = "std"))]
pub use std::alloc::Global;
