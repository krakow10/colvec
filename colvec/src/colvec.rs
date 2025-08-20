use crate::raw::TestRawColVec;

use core::ptr;

use allocator_api2::alloc::{Allocator, Global};

// [4444 2233 1PPP]

pub(crate) struct Test{
	field1:u8,
	field2:Option<u8>,
	field3:i16,
	field4:u32,
}

const OFFSET_FIELD_1:usize = 0;
const OFFSET_FIELD_2:usize = size_of::<u8>();
const OFFSET_FIELD_3:usize = size_of::<u8>() + size_of::<Option<u8>>();
const OFFSET_FIELD_4:usize = size_of::<u8>() + size_of::<Option<u8>>() + size_of::<i16>();

pub(crate) unsafe fn move_fields(
	ptr: *mut u8,
	old_capacity: usize,
	new_capacity: usize,
	len: usize,
){
	macro_rules! copy_field{
		($field:ident, $ty:ty, $offset:expr) => {
			unsafe {
				let src = ptr.add(old_capacity * $offset).cast::<$ty>();
				let dst = ptr.add(new_capacity * $offset).cast::<$ty>();
				ptr::copy_nonoverlapping(src, dst, len);
			}
		};
	}

	// the fields are moved in descending order, and the first field is skipped
	copy_field!(field4,u32,OFFSET_FIELD_4);
	copy_field!(field3,i16,OFFSET_FIELD_3);
	copy_field!(field2,Option<u8>,OFFSET_FIELD_2);
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
		write_field!(field1,u8,OFFSET_FIELD_1);
		write_field!(field2,Option<u8>,OFFSET_FIELD_2);
		write_field!(field3,i16,OFFSET_FIELD_3);
		write_field!(field4,u32,OFFSET_FIELD_4);
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
	(OFFSET_FIELD_1, u8, field1_slice, field1_slice_mut),
	(OFFSET_FIELD_2, Option<u8>, field2_slice, field2_slice_mut),
	(OFFSET_FIELD_3, i16, field3_slice, field3_slice_mut),
	(OFFSET_FIELD_4, u32, field4_slice, field4_slice_mut)
);

#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn it_works() {
		const LEN:usize = 9;
		let mut test=TestColVec::new();
		for _ in 0..LEN{
			test.push(Test{
				field1:1,
				field2:Some(2),
				field3:3,
				field4:4,
			});
		}

		let _1:Vec<_> =core::iter::repeat_n(1, LEN).collect();
		let _2:Vec<_> =core::iter::repeat_n(Some(2), LEN).collect();
		let _3:Vec<_> =core::iter::repeat_n(3, LEN).collect();
		let _4:Vec<_> =core::iter::repeat_n(4, LEN).collect();
		assert_eq!(&_1,test.field1_slice());
		assert_eq!(&_2,test.field2_slice());
		assert_eq!(&_3,test.field3_slice());
		assert_eq!(&_4,test.field4_slice());
	}
}
