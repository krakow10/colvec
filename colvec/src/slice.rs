use crate::raw::StructInfo;

use core::ptr;

pub struct RawColSlice<const N:usize,T:StructInfo<N>>([T]);

impl<const N:usize,T:StructInfo<N>> RawColSlice<N,T>{
	#[inline]
	#[must_use]
	pub const fn len(&self) -> usize {
		// This is cheating! .0 is never valid!
		// I see no other way to access the pointer metadata
		self.0.len()
	}
	pub const fn copy_from_col_slice(&mut self, src: &Self){
		#[track_caller]
        const fn len_mismatch_fail(_dst_len: usize, _src_len: usize) -> ! {
            panic!(
                "copy_from_slice: source slice length does not match destination slice length",
            )
        }
		if self.len() != src.len() {
			len_mismatch_fail(self.len(), src.len());
		}
		unsafe{
			T::FIELDS.move_fields(
				src as *const Self as *const u8,
				self as *mut Self as *mut u8,
				src_capacity,
				dst_capacity,
				0,
				self.len(),
			)
		}
	}
}
