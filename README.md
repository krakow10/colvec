Column Vec
==========

[![Latest version](https://img.shields.io/crates/v/colvec.svg)](https://crates.io/crates/colvec)
![License](https://img.shields.io/crates/l/colvec.svg)

`colvec` provides a derive macro which generates a Vec-like data structure, except with a transposed row / column layout.  This means you can take a slice across an entire column of a single struct field.  ColVec has the same struct size (`size_of::<Vec<T>>()`) and growth factor as Vec, and uses a single contiguous allocation.  ColVec can be smaller than Vec<Struct> when the Struct has padding, since no padding is needed in the transposed layout [^1].  ColVec can be faster to iterate than Vec<Struct> when only accessing a single struct field column because the unused data in the other fields do not occupy the cache line, and thus has reduced cache eviction.

[^1]: To ensure proper alignment, the minimum capacity is the item alignment.

## Example
```rust
#[derive(colvec::ColVec)]
struct Test{
	field1:u8,
	field2:Option<u8>,
	field3:i16,
	field4:u32,
}

let mut col=TestColVec::new();
col.push(Test{
	field1:1,
	field2:Some(2),
	field3:3,
	field4:4,
});
col.push(Test{
	field1:5,
	field2:Some(6),
	field3:7,
	field4:8,
});

assert_eq!(&[Some(2),Some(6)], col.field2_slice());
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
