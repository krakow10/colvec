use core::ptr::{self, NonNull};
use allocator_api2::alloc::{Allocator, Global};

struct Test{
	field1:u8,
	field2:Option<u8>,
	field3:i16,
	field4:u32,
}

struct TestColVec<A: Allocator = Global>{
	ptr: NonNull<u8>,
	cap: usize,
	len: usize,
	alloc: A,
}
