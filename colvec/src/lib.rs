pub use colvec_derive::ColVec;

pub mod colvec;
mod error;

// used from generated code

#[doc(hidden)]
pub mod alloc;
#[doc(hidden)]
pub mod fields;
#[doc(hidden)]
pub mod raw;

#[cfg(test)]
mod tests {
    use super::*;

	#[derive(ColVec)]
	struct Test{
		field1:u8,
		field2:i32,
	}

	#[test]
	fn it_works() {
		// let mut test=TestColVec::new();
		// test.push(Test{
		// 	field1:1,
		// 	field2:-1,
		// });
		// assert_eq!( 1, test.field1_slice()[0]);
		// assert_eq!(-1, test.field2_slice()[0]);
	}
}
