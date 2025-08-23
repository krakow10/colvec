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

	#[test]
	fn smoke_test_macro() {
		#[derive(ColVec)]
		struct Test{
			field0:u8,
			field1:i32,
		}

		let mut test=TestColVec::new();
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

		let mut test=ZSTColVec::new();
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

		let mut bugs=BugColVec::with_capacity(2);
		bugs.push(Bug{
			is_red:false,
			coolness:1,
		});

		let mut cool_bugs=BugColVec::with_capacity(1);
		cool_bugs.push(Bug{
			is_red:true,
			coolness:1337,
		});

		bugs.append(&mut cool_bugs);

		assert_eq!(&[false,true], bugs.is_red_slice());
		assert_eq!(&[1,1337], bugs.coolness_slice());
	}
}
