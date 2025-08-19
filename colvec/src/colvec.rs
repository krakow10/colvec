use crate::raw::TestRawColVec;

use core::ptr;
use core::mem::offset_of;

use allocator_api2::alloc::{Allocator, Global};

// [4444 2233 1PPP]

pub(crate) struct Test{
	field1:u8,
	field2:Option<u8>,
	field3:i16,
	field4:u32,
}

pub(crate) unsafe fn move_fields(
	ptr: *mut u8,
	old_capacity: usize,
	new_capacity: usize,
	len: usize,
){
	macro_rules! copy_field{
		($field:ident, $ty:ty) => {
			unsafe {
				let src = ptr.add(old_capacity * offset_of!(Test,$field)).cast::<$ty>();
				let dst = ptr.add(new_capacity * offset_of!(Test,$field)).cast::<$ty>();
				ptr::copy_nonoverlapping(src, dst, len);
			}
		};
	}

	// the fields are moved in offset-descending order, and the field at offset 0 is skipped
	copy_field!(field1,u8);
	copy_field!(field3,i16);
	copy_field!(field2,Option<u8>);
}

// Vec<Test> len 4 cap 4
// [4444 2233 1PPP][4444 2233 1PPP][4444 2233 1PPP][4444 2233 1PPP]

// TestColVec len 4 cap 4
// [4444][4444][4444][4444][22][22][22][22][33][33][33][33][1][1][1][1]

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
			($field:ident, $ty:ty) => {
				unsafe {
					let end = self.as_mut_ptr()
						.add(self.buf.capacity() * offset_of!(Test,$field))
						.cast::<$ty>()
						.add(len);
					ptr::write(end, value.$field);
				}
			};
		}
		write_field!(field1,u8);
		write_field!(field2,Option<u8>);
		write_field!(field3,i16);
		write_field!(field4,u32);
		self.len = len + 1;
	}
}

macro_rules! impl_field_access {
	($(($field:ident, $ty:ty, $immut:ident, $muta:ident)),*) => {
	    impl<A: Allocator> TestColVec<A>{
			$(
			    pub const fn $immut(&self) -> &[$ty] {
					unsafe {
						core::slice::from_raw_parts(
							self.as_ptr()
								.add(self.buf.capacity() * offset_of!(Test,$field))
								.cast::<$ty>(),
							self.len
						)
					}
				}
				pub const fn $muta(&mut self) -> &mut [$ty] {
					unsafe {
						core::slice::from_raw_parts_mut(
							self.as_mut_ptr()
								.add(self.buf.capacity() * offset_of!(Test,$field))
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
	(field1, u8, field1_slice, field1_slice_mut),
	(field2, Option<u8>, field2_slice, field2_slice_mut),
	(field3, i16, field3_slice, field3_slice_mut),
	(field4, u32, field4_slice, field4_slice_mut)
);

#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn it_works() {
		let mut test=TestColVec::new();
		for _ in 0..9{
			test.push(Test{
				field1:1,
				field2:None,
				field3:-1,
				field4:256,
			});
		}

		assert_eq!( 1, test.field1_slice()[0]);
		assert_eq!(-1, test.field3_slice()[0]);
		assert_eq!( 1, test.field1_slice()[8]);
		assert_eq!(-1, test.field3_slice()[8]);
	}
}
