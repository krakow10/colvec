use crate::raw::TestRawColVec;

use core::ptr;

use allocator_api2::alloc::{Allocator, Global};

// [4444 2233 1PPP]

pub(crate) struct Test{
	field0:u8,
	field1:Option<u8>,
	field2:i16,
	field3:u32,
}

// calculate array of offsets from sorted list of sizes
#[derive(Debug,Clone,Copy)]
struct Field{
	index:usize,
	size:usize,
	offset:usize,
}
const N:usize=4;
const FIELDS:[Field;N]=sizes_to_fields([
	size_of::<u8>(),
	size_of::<Option<u8>>(),
	size_of::<i16>(),
	size_of::<u32>(),
]);

const fn sizes_to_fields(sizes:[usize;N])->[Field;N]{
	// encode a u64 with size in the upper bits and index in the lower bits
	let mut index=0;
	let mut sides_encoded=[0u64;N];
	// don't have 4 billion struct fields or my code won't work
	assert!(N<u32::MAX as usize);
	while index<N{
		// don't have 4GB structs or my code won't work
		assert!(sizes[index]<u32::MAX as usize);
		let size=sizes[index] as u64;
		sides_encoded[index]=(size<<32)|(index as u64);
		index+=1;
	}

	// sort by size, and index as a tie breaker
	compile_time_sort::sort_u64_slice(&mut sides_encoded);

	let mut fields=[Field{index:0,size:0,offset:0};N];
	let mut i=0;
	let mut offset=0;
	while i<N{
		// decode back into index and size
		let (index,size)=(sides_encoded[N-i-1] as u32 as usize, (sides_encoded[N-i-1]>>32) as u32 as usize);
		fields[N-i-1]=Field{
			index,
			size,
			offset,
		};
		offset+=size;
		i+=1;
	}

	fields
}

const fn locate_field(index:usize) -> Field{
	let mut i=0;
	while i<N{
		if FIELDS[i].index==index{
			return FIELDS[i];
		}
		i+=1;
	}
	panic!("No field with index");
}

pub(crate) const unsafe fn move_fields(
	ptr: *mut u8,
	old_capacity: usize,
	new_capacity: usize,
	len: usize,
){
	// the fields are moved in descending-offset order, and the field at 0 offset is skipped
	let mut i=0;
	while i<N-1{
		unsafe {
			let src = ptr.add(old_capacity * FIELDS[i].offset);
			let dst = ptr.add(new_capacity * FIELDS[i].offset);
			let count = len * FIELDS[i].size;
			ptr::copy_nonoverlapping(src, dst, count);
		}
		i+=1;
	}
}

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

struct TestColVec<A: Allocator = Global>{
	buf: TestRawColVec<A>,
	len: usize,
}

impl TestColVec<Global>{
	#[inline]
	#[must_use]
	pub const fn new() -> Self {
		Self { buf: TestRawColVec::new(), len: 0 }
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
		write_field!(field0,u8,locate_field(0).offset);
		write_field!(field1,Option<u8>,locate_field(1).offset);
		write_field!(field2,i16,locate_field(2).offset);
		write_field!(field3,u32,locate_field(3).offset);
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
	(locate_field(0).offset, u8, field0_slice, field0_slice_mut),
	(locate_field(1).offset, Option<u8>, field1_slice, field1_slice_mut),
	(locate_field(2).offset, i16, field2_slice, field2_slice_mut),
	(locate_field(3).offset, u32, field3_slice, field3_slice_mut)
);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dbg() {
		let f=sizes_to_fields([1,2,3,4]);
		dbg!(f);
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
