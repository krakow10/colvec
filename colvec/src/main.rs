fn main(){}

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
			field0:1,
			field1:-1,
		});

		test.field0_slice_mut().sort();
		test.field1_slice_mut().sort();

		assert_eq!( 1, test.field0_slice()[0]);
		assert_eq!(-1, test.field1_slice()[0]);
	}
}
