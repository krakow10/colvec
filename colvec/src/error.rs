use core::alloc::Layout;

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TryReserveError {
	kind: TryReserveErrorKind,
}

impl TryReserveError {
	/// Details about the allocation that caused the error
	#[inline]
	#[must_use]
	pub fn kind(&self) -> TryReserveErrorKind {
		self.kind.clone()
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveErrorKind {
	/// Error due to the computed capacity exceeding the collection's maximum
	/// (usually `isize::MAX` bytes).
	CapacityOverflow,

	/// The memory allocator returned an error
	AllocError {
		/// The layout of allocation request that failed
		layout: Layout,
	}
}

impl From<TryReserveErrorKind> for TryReserveError {
	#[inline]
	fn from(kind: TryReserveErrorKind) -> Self {
		Self { kind }
	}
}
