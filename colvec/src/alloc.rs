#[cfg(not(feature = "nightly"))]
pub use allocator_api2::alloc::Allocator;
#[cfg(feature = "nightly")]
pub use core::alloc::Allocator;

#[cfg(all(not(feature = "nightly"),feature = "std"))]
pub use allocator_api2::alloc::Global;
#[cfg(all(feature = "nightly",feature = "std"))]
pub use std::alloc::Global;
