use core::alloc::{Layout};
use core::cmp;
use core::hint;
use core::ptr::NonNull;
use core::num::NonZero;

use crate::error::TryReserveError;
use crate::error::TryReserveErrorKind::*;

use allocator_api2::alloc::{Allocator, Global};

// le type
use crate::colvec::Test;
use crate::colvec::move_fields;

// One central function responsible for reporting capacity overflows. This'll
// ensure that the code generation related to these panics is minimal as there's
// only one location which panics rather than a bunch throughout the module.
#[track_caller]
fn capacity_overflow() -> ! {
	panic!("capacity overflow");
}

pub(crate) struct TestRawColVec<A: Allocator = Global> {
	inner: TestRawColVecInner<A>,
}

struct TestRawColVecInner<A: Allocator = Global> {
	ptr: NonNull<u8>,
	cap: usize,
	alloc: A,
}

impl TestRawColVec<Global> {
	#[must_use]
	pub(crate) const fn new() -> Self {
		Self::new_in(Global)
	}
	// #[must_use]
	// #[inline]
	// #[track_caller]
	// pub(crate) fn with_capacity(capacity: usize) -> Self {
	// 	Self { inner: TestRawColVecInner::with_capacity(capacity, Layout::new::<Test>()) }
	// }
}


impl TestRawColVecInner<Global> {
	// #[must_use]
	// #[inline]
	// #[track_caller]
	// fn with_capacity(capacity: usize, elem_layout: Layout) -> Self {
	//     match Self::try_allocate_in(capacity, AllocInit::Uninitialized, Global, elem_layout) {
	//         Ok(res) => res,
	//         Err(err) => handle_error(err),
	//     }
	// }
}

// Tiny Vecs are dumb. Skip to:
// - 8 if the element size is 1, because any heap allocators is likely
//   to round up a request of less than 8 bytes to at least 8 bytes.
// - 4 if elements are moderate-sized (<= 1 KiB).
// - 1 otherwise, to avoid wasting too much space for very short Vecs.
const fn min_non_zero_cap(size: usize, align: usize) -> usize {
	let min_from_size=if size == 1 {
		8
	} else if size <= 1024 {
		4
	} else {
		1
	};
	// the transposed layout must have correct alignment for every field
	let min_from_align=align;
	// max of mins
	if min_from_align<min_from_size{
		min_from_size
	}else{
		min_from_align
	}
}

impl<A: Allocator> TestRawColVec<A> {
	pub(crate) const MIN_NON_ZERO_CAP: usize = min_non_zero_cap(size_of::<Test>(),align_of::<Test>());

	#[inline]
	pub(crate) const fn new_in(alloc: A) -> Self {
		Self { inner: TestRawColVecInner::new_in(alloc, NonZero::new(align_of::<Test>()).unwrap()) }
	}
	// #[inline]
	// #[track_caller]
	// pub(crate) fn with_capacity_in(capacity: usize, alloc: A) -> Self {
	//     Self {
	//         inner: RawVecInner::with_capacity_in(capacity, alloc, Layout::new::<Test>()),
	//     }
	// }
	#[inline]
	pub(crate) const fn capacity(&self) -> usize {
		self.inner.capacity(size_of::<Test>())
	}
	/// Gets a raw pointer to the start of the allocation. Note that this is
	/// `Unique::dangling()` if `capacity == 0` or `T` is zero-sized. In the former case, you must
	/// be careful.
	#[inline]
	pub(crate) const fn ptr(&self) -> *mut u8 {
		self.inner.ptr.as_ptr()
	}
	#[inline(never)]
	#[track_caller]
	pub(crate) fn grow_one(&mut self) {
		self.inner.grow_one(Layout::new::<Test>())
	}
}

impl<A: Allocator> TestRawColVecInner<A> {
	#[inline]
	const fn new_in(alloc: A, align: NonZero<usize>) -> Self {
		let ptr = NonNull::without_provenance(align);
		// `cap: 0` means "unallocated". zero-sized types are ignored.
		Self { ptr, cap: 0, alloc }
	}
	#[inline]
	const fn capacity(&self, elem_size: usize) -> usize {
		if elem_size == 0 { usize::MAX } else { self.cap }
	}
	#[inline]
	#[track_caller]
	fn grow_one(&mut self, elem_layout: Layout) {
		if let Err(err) = self.grow_amortized(self.cap, 1, elem_layout) {
			handle_error(err);
		}
	}
	#[inline]
	fn current_memory(&self, elem_layout: Layout) -> Option<(NonNull<u8>, Layout)> {
		if elem_layout.size() == 0 || self.cap == 0 {
			None
		} else {
			// We could use Layout::array here which ensures the absence of isize and usize overflows
			// and could hypothetically handle differences between stride and size, but this memory
			// has already been allocated so we know it can't overflow and currently Rust does not
			// support such types. So we can do better by skipping some checks and avoid an unwrap.
			unsafe {
				let alloc_size = elem_layout.size().unchecked_mul(self.cap);
				let layout = Layout::from_size_align_unchecked(alloc_size, elem_layout.align());
				Some((self.ptr.into(), layout))
			}
		}
	}
	#[inline]
	unsafe fn set_ptr_and_cap(&mut self, ptr: NonNull<[u8]>, cap: usize) {
		// Allocators currently return a `NonNull<[u8]>` whose length matches
		// the size requested. If that ever changes, the capacity here should
		// change to `ptr.len() / size_of::<T>()`.
		self.ptr = ptr.cast();
		self.cap = cap;
	}
	fn grow_amortized(
		&mut self,
		len: usize,
		additional: usize,
		elem_layout: Layout,
	) -> Result<(), TryReserveError> {
		// This is ensured by the calling contexts.
		debug_assert!(additional > 0);

		if elem_layout.size() == 0 {
			// Since we return a capacity of `usize::MAX` when `elem_size` is
			// 0, getting to here necessarily means the `RawVec` is overfull.
			return Err(CapacityOverflow.into());
		}

		// Nothing we can really do about these checks, sadly.
		let required_cap = len.checked_add(additional).ok_or(CapacityOverflow)?;

		// This guarantees exponential growth. The doubling cannot overflow
		// because `cap <= isize::MAX` and the type of `cap` is `usize`.
		let cap = cmp::max(self.cap * 2, required_cap);
		let cap = cmp::max(min_non_zero_cap(elem_layout.size(),elem_layout.align()), cap);

		let new_layout = layout_array(cap, elem_layout)?;

		let ptr = finish_grow(
			new_layout,
			self.current_memory(elem_layout),
			&mut self.alloc,
			self.cap,
			cap,
			len,
		)?;
		// SAFETY: finish_grow would have resulted in a capacity overflow if we tried to allocate more than `isize::MAX` items

		unsafe { self.set_ptr_and_cap(ptr, cap) };
		Ok(())
	}
}

// not marked inline(never) since we want optimizers to be able to observe the specifics of this
// function, see tests/codegen/vec-reserve-extend.rs.
#[cold]
fn finish_grow<A>(
	new_layout: Layout,
	current_memory: Option<(NonNull<u8>, Layout)>,
	alloc: &mut A,
	old_capacity: usize,
	new_capacity: usize,
	len: usize,
) -> Result<NonNull<[u8]>, TryReserveError>
where
	A: Allocator,
{
	alloc_guard(new_layout.size())?;

	if let Some((ptr, old_layout)) = current_memory {
		debug_assert_eq!(old_layout.align(), new_layout.align());
		let memory = unsafe {
			// The allocator checks for alignment equality
			hint::assert_unchecked(old_layout.align() == new_layout.align());
			alloc.grow(ptr, old_layout, new_layout)
		};
		let Ok(region) = memory else{
			return Err(AllocError { layout: new_layout }.into());
		};

		unsafe{ move_fields(ptr.as_ptr(), old_capacity, new_capacity, len) }

		Ok(region)
	} else {
		alloc.allocate(new_layout)
			.map_err(|_| AllocError { layout: new_layout }.into())
	}
}

#[cold]
#[track_caller]
fn handle_error(e: TryReserveError) -> ! {
	match e.kind() {
		CapacityOverflow => capacity_overflow(),
		AllocError { layout, .. } => allocator_api2::alloc::handle_alloc_error(layout),
	}
}


// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects.
// * We don't overflow `usize::MAX` and actually allocate too little.
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit and 16-bit we need to add
// an extra guard for this in case we're running on a platform which can use
// all 4GB in user-space, e.g., PAE or x32.
#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), TryReserveError> {
	if usize::BITS < 64 && alloc_size > isize::MAX as usize {
		Err(CapacityOverflow.into())
	} else {
		Ok(())
	}
}

struct LayoutError;

#[inline]
const fn repeat_packed(layout: &Layout, n: usize) -> Result<Layout, LayoutError> {
	if let Some(size) = layout.size().checked_mul(n) {
		// The safe constructor is called here to enforce the isize size limit.
		Ok(unsafe{ Layout::from_size_align_unchecked(size, layout.align()) })
	} else {
		Err(LayoutError)
	}
}

#[inline]
const fn repeat(layout: &Layout, n: usize) -> Result<(Layout, usize), LayoutError> {
	let padded = layout.pad_to_align();
	if let Ok(repeated) = repeat_packed(&padded, n) {
		Ok((repeated, padded.size()))
	} else {
		Err(LayoutError)
	}
}

#[inline]
fn layout_array(cap: usize, elem_layout: Layout) -> Result<Layout, TryReserveError> {
	repeat(&elem_layout, cap).map(|(layout, _pad)| layout).map_err(|_| CapacityOverflow.into())
}
