#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

fn main(){}

// Idea: multi-slice like Vec<T> Derefs to &[T]
// RawColVec Derefs to &RawColSlice
// &RawColSlice has .field1_slice
// RawColVec DerefMuts to &mut RawColSlice
// &mut RawColSlice has .field1_slice_mut
//
// RawColSlice is a ZST but is !Sized to get the fat pointer

#[cfg(test)]
mod tests {
	use ::colvec::*;

	#[cfg(not(feature = "std"))]
	mod global {
		extern crate alloc as core_alloc;
		use core_alloc::alloc::{alloc, dealloc, Layout};
		use core::ptr::NonNull;

		use ::colvec::alloc::{Allocator,AllocError};

		#[derive(Copy, Clone)]
		pub struct Global;

		unsafe impl Allocator for Global {
			#[inline]
			fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
				let non_null=NonNull::new(unsafe{alloc(layout)}).ok_or(AllocError)?;
				Ok(NonNull::slice_from_raw_parts(non_null, layout.size()))
			}
			#[inline]
			unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
				unsafe {dealloc(ptr.as_ptr(), layout) }
			}
		}
		impl Default for Global {
			#[inline]
			fn default() -> Self {
				Global
			}
		}
	}

	#[test]
	fn smoke_test_macro() {
		#[derive(ColVec)]
		struct Test{
			field0:u8,
			field1:i32,
		}

		#[cfg(feature = "std")]
		let mut test=TestColVec::new();
		#[cfg(not(feature = "std"))]
		let mut test=TestColVec::new_in(global::Global);

		test.push(Test{
			field0:255,
			field1:-1,
		});
		test.push(Test{
			field0:1,
			field1:-2,
		});

		test.field0_slice_mut().sort();
		test.field1_slice_mut().sort();

		assert_eq!(&[1,255], test.field0_slice());
		assert_eq!(&[-2,-1], test.field1_slice());
	}

	#[test]
	fn test_zst(){
		#[derive(ColVec)]
		struct ZST{}

		#[cfg(feature = "std")]
		let mut test=ZSTColVec::new();
		#[cfg(not(feature = "std"))]
		let mut test=ZSTColVec::new_in(global::Global);

		test.push(ZST{});
		test.push(ZST{});

		assert_eq!(2, test.len());
	}

	#[test]
	fn test_append() {
		#[derive(ColVec)]
		struct Bug{
			is_red:bool,
			coolness:u64,
		}

		#[cfg(feature = "std")]
		let mut bugs=BugColVec::with_capacity(2);
		#[cfg(not(feature = "std"))]
		let mut bugs=BugColVec::with_capacity_in(2,global::Global);
		bugs.push(Bug{
			is_red:false,
			coolness:1,
		});

		#[cfg(feature = "std")]
		let mut cool_bugs=BugColVec::with_capacity(1);
		#[cfg(not(feature = "std"))]
		let mut cool_bugs=BugColVec::with_capacity_in(1,global::Global);
		cool_bugs.push(Bug{
			is_red:true,
			coolness:1337,
		});

		bugs.append(&mut cool_bugs);

		assert_eq!(&[false,true], bugs.is_red_slice());
		assert_eq!(&[1,1337], bugs.coolness_slice());
	}
}
