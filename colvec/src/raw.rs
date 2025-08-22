use core::alloc::Layout;
use core::cmp;
use core::hint;
use core::marker::PhantomData;
use core::num::NonZero;
use core::ptr::NonNull;

use crate::alloc::Allocator;
use crate::error::TryReserveError;
use crate::error::TryReserveErrorKind::*;
use crate::fields::Fields;

// One central function responsible for reporting capacity overflows. This'll
// ensure that the code generation related to these panics is minimal as there's
// only one location which panics rather than a bunch throughout the module.
#[track_caller]
fn capacity_overflow() -> ! {
	panic!("capacity overflow");
}

enum AllocInit {
	/// The contents of the new memory are uninitialized.
	Uninitialized,
	/// The new memory is guaranteed to be zeroed.
	Zeroed,
}

pub struct RawColVec<const N:usize, T: StructInfo<N>, A: Allocator> {
	inner: RawColVecInner<A>,
	_marker: PhantomData<T>,
}
unsafe impl<const N:usize, T: Send + StructInfo<N>, A: Allocator> Send for RawColVec<N, T, A> {}
unsafe impl<const N:usize, T: Sync + StructInfo<N>, A: Allocator> Sync for RawColVec<N, T, A> {}

struct RawColVecInner<A: Allocator> {
	ptr: NonNull<u8>,
	cap: usize,
	alloc: A,
}

// TODO: don't do this
pub trait StructInfo<const N:usize> {
	const LAYOUT:Layout;
	const FIELDS:Fields::<N>;
}

// Tiny Vecs are dumb. Skip to:
// - 8 if the element size is 1, because any heap allocators is likely
//   to round up a request of less than 8 bytes to at least 8 bytes.
// - 4 if elements are moderate-sized (<= 1 KiB).
// - 1 otherwise, to avoid wasting too much space for very short Vecs.
const fn min_non_zero_cap(size: usize) -> usize {
	if size == 1 {
		8
	} else if size <= 1024 {
		4
	} else {
		1
	}
}

impl<const N:usize, T: StructInfo<N>, A: Allocator> RawColVec<N, T, A> {
	#[inline]
	pub const fn new_in(alloc: A) -> Self {
		Self {
			inner: RawColVecInner::new_in(alloc, NonZero::new(T::LAYOUT.align()).unwrap()),
			_marker: PhantomData,
		}
	}
	#[inline]
	#[track_caller]
	pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
		Self {
			inner: RawColVecInner::with_capacity_in(capacity, alloc, T::LAYOUT),
			_marker: PhantomData,
		}
	}
	#[inline]
	pub const fn capacity(&self) -> usize {
		self.inner.capacity(T::LAYOUT.size())
	}
	#[inline]
	#[track_caller]
	pub fn reserve(&mut self, len: usize, additional: usize) {
		self.inner.reserve(len, additional, T::LAYOUT, &T::FIELDS)
	}
	/// Gets a raw pointer to the start of the allocation. Note that this is
	/// `Unique::dangling()` if `capacity == 0` or `T` is zero-sized. In the former case, you must
	/// be careful.
	#[inline]
	pub const fn ptr(&self) -> *mut u8 {
		self.inner.ptr.as_ptr()
	}
	#[inline(never)]
	#[track_caller]
	pub fn grow_one(&mut self) {
		self.inner.grow_one(T::LAYOUT,&T::FIELDS)
	}
}

impl<const N:usize, T: StructInfo<N>, A: Allocator> Drop for RawColVec<N, T, A> {
	/// Frees the memory owned by the `RawVec` *without* trying to drop its contents.
	fn drop(&mut self) {
		// SAFETY: We are in a Drop impl, self.inner will not be used again.
		unsafe { self.inner.deallocate(T::LAYOUT) }
	}
}

impl<A: Allocator> RawColVecInner<A> {
	#[inline]
	const fn new_in(alloc: A, align: NonZero<usize>) -> Self {
		let ptr = NonNull::without_provenance(align);
		// `cap: 0` means "unallocated". zero-sized types are ignored.
		Self { ptr, cap: 0, alloc }
	}
	#[inline]
	#[track_caller]
	fn with_capacity_in(capacity: usize, alloc: A, elem_layout: Layout) -> Self {
		match Self::try_allocate_in(capacity, AllocInit::Uninitialized, alloc, elem_layout) {
			Ok(this) => {
				unsafe {
					// Make it more obvious that a subsequent Vec::reserve(capacity) will not allocate.
					hint::assert_unchecked(!this.needs_to_grow(0, capacity, elem_layout));
				}
				this
			}
			Err(err) => handle_error(err),
		}
	}
	fn try_allocate_in(
		capacity: usize,
		init: AllocInit,
		alloc: A,
		elem_layout: Layout,
	) -> Result<Self, TryReserveError> {
		// capacity must be a multiple of alignment
		let capacity = capacity.next_multiple_of(elem_layout.align());
		// We avoid `unwrap_or_else` here because it bloats the amount of
		// LLVM IR generated.
		let layout = match layout_colvec(capacity, elem_layout) {
			Ok(layout) => layout,
			Err(_) => return Err(CapacityOverflow.into()),
		};

		// Don't allocate here because `Drop` will not deallocate when `capacity` is 0.
		if layout.size() == 0 {
			return Ok(Self::new_in(alloc, unsafe{NonZero::new_unchecked(elem_layout.align())}));
		}

		if let Err(err) = alloc_guard(layout.size()) {
			return Err(err);
		}

		let result = match init {
			AllocInit::Uninitialized => alloc.allocate(layout),
			AllocInit::Zeroed => alloc.allocate_zeroed(layout),
		};
		let ptr = match result {
			Ok(ptr) => ptr,
			Err(_) => return Err(AllocError { layout }.into()),
		};

		// Allocators currently return a `NonNull<[u8]>` whose length
		// matches the size requested. If that ever changes, the capacity
		// here should change to `ptr.len() / size_of::<T>()`.
		Ok(Self {
			ptr: ptr.cast(),
			cap: capacity,
			alloc,
		})
	}
	#[inline]
	const fn capacity(&self, elem_size: usize) -> usize {
		if elem_size == 0 { usize::MAX } else { self.cap }
	}
	#[inline]
	#[track_caller]
	fn reserve<const N:usize>(&mut self, len: usize, additional: usize, elem_layout: Layout, fields: &Fields<N>) {
		// Callers expect this function to be very cheap when there is already sufficient capacity.
		// Therefore, we move all the resizing and error-handling logic from grow_amortized and
		// handle_reserve behind a call, while making sure that this function is likely to be
		// inlined as just a comparison and a call if the comparison fails.
		#[cold]
		fn do_reserve_and_handle<const N:usize, A: Allocator>(
			slf: &mut RawColVecInner<A>,
			len: usize,
			additional: usize,
			elem_layout: Layout,
			fields: &Fields<N>,
		) {
			if let Err(err) = slf.grow_amortized(len, additional, elem_layout, fields) {
				handle_error(err);
			}
		}

		if self.needs_to_grow(len, additional, elem_layout) {
			do_reserve_and_handle(self, len, additional, elem_layout, fields);
		}
	}
	#[inline]
	#[track_caller]
	fn grow_one<const N:usize>(&mut self, elem_layout: Layout, fields: &Fields<N>) {
		if let Err(err) = self.grow_amortized(self.cap, 1, elem_layout, fields) {
			handle_error(err);
		}
	}
	#[inline]
	fn needs_to_grow(&self, len: usize, additional: usize, elem_layout: Layout) -> bool {
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
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
	fn grow_amortized<const N:usize>(
		&mut self,
		len: usize,
		additional: usize,
		elem_layout: Layout,
		fields: &Fields<N>,
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
		let cap = cmp::max(min_non_zero_cap(elem_layout.size()), cap);
		// cap must be a multiple of align due to using the unpadded elem_layout
		// for the allocation layout calculation in `layout_colvec`.
		let cap = cap.next_multiple_of(elem_layout.align());

		let new_layout = layout_colvec(cap, elem_layout)?;

		let ptr = finish_grow(
			new_layout,
			self.current_memory(elem_layout),
			&mut self.alloc,
			fields,
			self.cap,
			cap,
			len,
		)?;
		// SAFETY: finish_grow would have resulted in a capacity overflow if we tried to allocate more than `isize::MAX` items

		unsafe { self.set_ptr_and_cap(ptr, cap) };
		Ok(())
	}
	/// # Safety
	///
	/// This function deallocates the owned allocation, but does not update `ptr` or `cap` to
	/// prevent double-free or use-after-free. Essentially, do not do anything with the caller
	/// after this function returns.
	/// Ideally this function would take `self` by move, but it cannot because it exists to be
	/// called from a `Drop` impl.
	unsafe fn deallocate(&mut self, elem_layout: Layout) {
		if let Some((ptr, layout)) = self.current_memory(elem_layout) {
			unsafe {
				self.alloc.deallocate(ptr, layout);
			}
		}
	}
}

// not marked inline(never) since we want optimizers to be able to observe the specifics of this
// function, see tests/codegen/vec-reserve-extend.rs.
#[cold]
fn finish_grow<const N:usize,A>(
	new_layout: Layout,
	current_memory: Option<(NonNull<u8>, Layout)>,
	alloc: &mut A,
	fields: &Fields<N>,
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

		unsafe{ fields.grow_fields(ptr.as_ptr(), old_capacity, new_capacity, len) }

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
fn layout_colvec(cap: usize, elem_layout: Layout) -> Result<Layout, TryReserveError> {
	repeat_packed(&elem_layout, cap).map_err(|_| CapacityOverflow.into())
}
