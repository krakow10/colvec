use crate::alloc::{Allocator,Global};
use crate::fields::Fields;
use crate::raw::{SmuggleOuter,RawColVec};

use core::alloc::Layout;
use core::ptr;

// [4444 2233 1PPP]

pub struct Test{
	field0:u8,
	field1:Option<u8>,
	field2:i16,
	field3:u32,
}

const fn unpadded_elem_layout() -> Layout {
	let size = size_of::<u8>() + size_of::<Option<u8>>() + size_of::<i16>() + size_of::<u32>();
	let align = align_of::<Test>();
	unsafe { Layout::from_size_align_unchecked(size, align) }
}

impl SmuggleOuter for Test{
	const LAYOUT: Layout = unpadded_elem_layout();
	unsafe fn move_fields(
		ptr: *mut u8,
		old_capacity: usize,
		new_capacity: usize,
		len: usize,
	) {
		unsafe { FIELDS.move_fields(ptr, old_capacity, new_capacity, len) }
	}
}

// calculate array of offsets from sorted list of sizes
const N:usize=4;
const FIELDS:Fields<N> =Fields::from_sizes([
	size_of::<u8>(),
	size_of::<Option<u8>>(),
	size_of::<i16>(),
	size_of::<u32>(),
]);

// memory layout diagrams

// Vec<Test> len 4 cap 4
// [4444 2233 1PPP][4444 2233 1PPP][4444 2233 1PPP][4444 2233 1PPP]

// TestColVec len 4 cap 4
// 111122222222333333334444444444444444

// notice how field2 overlaps when the capacity is increased

// TestColVec len 4 cap 8
// 1111EEEE22222222EEEEEEEE33333333EEEEEEEE4444444444444444EEEEEEEEEEEEEEEE

// if we sort the fields by size decreasing, this cannot happen.

// TestColVec len 4 cap 4
// 444444444444444422222222333333331111

// TestColVec len 4 cap 8
// 4444444444444444EEEEEEEEEEEEEEEE22222222EEEEEEEE33333333EEEEEEEE1111EEEE

pub struct TestColVec<A: Allocator = Global>{
	buf: RawColVec<Test, A>,
	len: usize,
}

impl TestColVec<Global>{
	#[inline]
	#[must_use]
	pub const fn new() -> Self {
		Self { buf: RawColVec::new(), len: 0 }
	}
}
impl<A: Allocator> TestColVec<A>{
	pub const fn capacity(&self) -> usize {
		self.buf.capacity()
	}
	#[inline]
	const fn as_ptr(&self) -> *const u8 {
		// We shadow the slice method of the same name to avoid going through
		// `deref`, which creates an intermediate reference.
		self.buf.ptr()
	}
	#[inline]
	const fn as_mut_ptr(&mut self) -> *mut u8 {
		// We shadow the slice method of the same name to avoid going through
		// `deref_mut`, which creates an intermediate reference.
		self.buf.ptr()
	}
	pub fn push(&mut self, value: Test){
		// Inform codegen that the length does not change across grow_one().
		let len = self.len;
		// This will panic or abort if we would allocate > isize::MAX bytes
		// or if the length increment would overflow for zero-sized types.
		if len == self.buf.capacity() {
			self.buf.grow_one();
		}
		macro_rules! write_field{
			($field:ident, $ty:ty, $offset:expr) => {
				unsafe {
					let end = self.as_mut_ptr()
						.add(self.buf.capacity() * $offset)
						.cast::<$ty>()
						.add(len);
					ptr::write(end, value.$field);
				}
			};
		}
		write_field!(field0,u8,FIELDS.offset_of(0));
		write_field!(field1,Option<u8>,FIELDS.offset_of(1));
		write_field!(field2,i16,FIELDS.offset_of(2));
		write_field!(field3,u32,FIELDS.offset_of(3));
		self.len = len + 1;
	}
}

macro_rules! impl_field_access {
	($(($offset:expr, $ty:ty, $slice:ident, $slice_mut:ident)),*) => {
		impl<A: Allocator> TestColVec<A>{
			$(
				pub const fn $slice(&self) -> &[$ty] {
					unsafe {
						core::slice::from_raw_parts(
							self.as_ptr()
								.add(self.buf.capacity() * $offset)
								.cast::<$ty>(),
							self.len
						)
					}
				}
				pub const fn $slice_mut(&mut self) -> &mut [$ty] {
					unsafe {
						core::slice::from_raw_parts_mut(
							self.as_mut_ptr()
								.add(self.buf.capacity() * $offset)
								.cast::<$ty>(),
							self.len
						)
					}
				}
			)*
		}
	};
}

impl_field_access!(
	(FIELDS.offset_of(0), u8, field0_slice, field0_slice_mut),
	(FIELDS.offset_of(1), Option<u8>, field1_slice, field1_slice_mut),
	(FIELDS.offset_of(2), i16, field2_slice, field2_slice_mut),
	(FIELDS.offset_of(3), u32, field3_slice, field3_slice_mut)
);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_fields() {
		let f=Fields::from_sizes([4,2,3,1]);
		assert_eq!(f.offset_of(0),0);// field0 size is 4 and offset is 0
		assert_eq!(f.offset_of(1),4+3);// field1 size is 2 and offset is 4+3
		assert_eq!(f.offset_of(2),4);// field2 size is 3 and offset is 4
		assert_eq!(f.offset_of(3),4+3+2);// field3 size is 1 and offset is 4+3+2
	}

	#[test]
	fn it_works() {
		const LEN:usize = 9;
		let mut test=TestColVec::new();
		for _ in 0..LEN{
			test.push(Test{
				field0:0,
				field1:Some(1),
				field2:2,
				field3:3,
			});
		}

		let _0:Vec<_> =core::iter::repeat_n(0, LEN).collect();
		let _1:Vec<_> =core::iter::repeat_n(Some(1), LEN).collect();
		let _2:Vec<_> =core::iter::repeat_n(2, LEN).collect();
		let _3:Vec<_> =core::iter::repeat_n(3, LEN).collect();
		assert_eq!(&_0,test.field0_slice());
		assert_eq!(&_1,test.field1_slice());
		assert_eq!(&_2,test.field2_slice());
		assert_eq!(&_3,test.field3_slice());
	}
}
