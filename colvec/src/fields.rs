pub struct Fields<const N:usize>{
	field_id_to_offset:[usize;N],
	sorted_fields:[Field;N],
}

#[derive(Clone,Copy)]
struct Field{
	size:usize,
	offset:usize,
}

impl<const N:usize> Fields<N>{
	pub const fn from_sizes(sizes:[usize;N])->Self{
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

		let mut field_id_to_offset=[0;N];
		let mut sorted_fields=[Field{size:0,offset:0};N];
		let mut i=0;
		let mut offset=0;
		while i<N{
			// decode back into index and size
			let (index,size)=(sides_encoded[N-i-1] as u32 as usize, (sides_encoded[N-i-1]>>32) as u32 as usize);
			field_id_to_offset[index]=offset;
			sorted_fields[N-i-1]=Field{
				size,
				offset,
			};
			offset+=size;
			i+=1;
		}

		Fields{
			field_id_to_offset,
			sorted_fields,
		}
	}
	pub const fn size(&self)->usize{
		let mut size=0;
		let mut i=0;
		while i<N{
			size+=self.sorted_fields[i].size;
			i+=1;
		}
		size
	}
	pub const fn offset_of(&self,index:usize)->usize{
		self.field_id_to_offset[index]
	}
	pub const unsafe fn move_fields(
		&self,
		ptr: *mut u8,
		old_capacity: usize,
		new_capacity: usize,
		len: usize,
	){
		// the fields are moved in descending-offset order, and the field at 0 offset is skipped
		let mut i=0;
		while i<N-1{
			unsafe {
				let src = ptr.add(old_capacity * self.sorted_fields[i].offset);
				let dst = ptr.add(new_capacity * self.sorted_fields[i].offset);
				let count = len * self.sorted_fields[i].size;
				core::ptr::copy_nonoverlapping(src, dst, count);
			}
			i+=1;
		}
	}
}
