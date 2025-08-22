#[cfg(not(test))]
use proc_macro::TokenStream;
#[cfg(test)]
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[cfg(not(test))]
#[proc_macro_derive(ColVec)]
pub fn colvec_derive(input:TokenStream)->TokenStream{
	let input:DeriveInput=syn::parse_macro_input!(input);
	colvec_derive_inner(input)
}

fn colvec_derive_inner(input:DeriveInput)->TokenStream{
	match input.data{
		syn::Data::Struct(syn::DataStruct{fields:syn::Fields::Named(fields_named),..})=>derive_struct(input.ident,input.vis,fields_named),
		_=>unimplemented!("Only structs are supported"),
	}
}

fn derive_struct(ident:syn::Ident,vis:syn::Visibility,fields:syn::FieldsNamed)->TokenStream{
	let colvec_ident_string=format!("{ident}ColVec");
	let colvec_ident=syn::Ident::new(&colvec_ident_string,ident.span());

	let fields_count=fields.named.len();
	let colvec = quote!{
		#vis struct #colvec_ident<A: ::colvec::alloc::Allocator = ::colvec::alloc::Global>{
			buf: ::colvec::raw::RawColVec<#fields_count, #ident, A>,
			len: usize,
		}
	};

	let global = quote! {
		impl #colvec_ident<::colvec::alloc::Global>{
			#[inline]
			#[must_use]
			pub const fn new() -> Self {
				Self { buf: ::colvec::raw::RawColVec::new_in(::colvec::alloc::Global), len: 0 }
			}
			#[inline]
			#[must_use]
			#[track_caller]
			pub fn with_capacity(capacity: usize) -> Self {
				Self::with_capacity_in(capacity, ::colvec::alloc::Global)
			}
		}
	};

	// this trait smuggles information about the input type into RawColVec and RawColVecInner
	let fields_types=fields.named.iter().map(|field|field.ty.clone());
	let struct_info = quote! {
		impl ::colvec::raw::StructInfo<#fields_count> for #ident{
			const LAYOUT: ::core::alloc::Layout = unsafe {
				let size = Self::FIELDS.size();
				let align = align_of::<#ident>();
				::core::alloc::Layout::from_size_align_unchecked(size, align)
			};
			const FIELDS: ::colvec::fields::Fields<#fields_count> = ::colvec::fields::Fields::from_sizes([
				#(size_of::<#fields_types>()),*
			]);
		}
	};

	let field_indices=0..fields_count;
	let field_types=fields.named.iter().map(|field|field.ty.clone());
	let field_idents=fields.named.iter().map(|field|field.ident.as_ref().unwrap().clone());
	let impls = quote! {
		impl<A: ::colvec::alloc::Allocator> #colvec_ident<A>{
			#[inline]
			pub const fn new_in(alloc: A) -> Self {
				Self { buf: ::colvec::raw::RawColVec::new_in(alloc), len: 0 }
			}
			#[inline]
			#[track_caller]
			pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
				Self { buf: ::colvec::raw::RawColVec::with_capacity_in(capacity, alloc), len: 0 }
			}
			#[inline]
			pub const fn capacity(&self) -> usize {
				self.buf.capacity()
			}
			#[track_caller]
			pub fn reserve(&mut self, additional: usize) {
				self.buf.reserve(self.len, additional);
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
			#[inline]
			pub unsafe fn set_len(&mut self, new_len: usize) {
				debug_assert!(new_len <= self.capacity());

				self.len = new_len;
			}
			pub fn push(&mut self, value: #ident){
				// Inform codegen that the length does not change across grow_one().
				let len = self.len;
				// This will panic or abort if we would allocate > isize::MAX bytes
				// or if the length increment would overflow for zero-sized types.
				if len == self.buf.capacity() {
					self.buf.grow_one();
				}
				unsafe {
					#(
						let end = self.as_mut_ptr()
							.add(self.buf.capacity() * <#ident as ::colvec::raw::StructInfo<#fields_count>>::FIELDS.offset_of(#field_indices))
							.cast::<#field_types>()
							.add(len);
						::core::ptr::write(end, value.#field_idents);
					)*
				}
				self.len = len + 1;
			}
			#[inline]
			#[track_caller]
			pub fn append(&mut self, other: &mut Self) {
				unsafe {
					self.append_elements(other);
					other.set_len(0);
				}
			}

			/// Appends elements to `self` from other buffer.
			#[inline]
			#[track_caller]
			unsafe fn append_elements(&mut self, other: &Self) {
				let count = other.len();
				self.reserve(count);
				let len = self.len();
				unsafe {
					<#ident as ::colvec::raw::StructInfo<#fields_count>>::FIELDS.move_fields(
						other.as_ptr(),
						self.as_mut_ptr(),
						other.capacity(),
						self.capacity(),
						len,
						count,
					)
				}
				self.len += count;
			}
		   #[inline]
			pub const fn len(&self) -> usize {
				self.len
			}
		}
	};

	let field_indices=0..fields_count;
	let field_types=fields.named.iter().map(|field|field.ty.clone());
	let field_slice_fn_idents=fields.named.iter().map(|field|{
		let ident=field.ident.as_ref().unwrap();
		let slice_ident=format!("{ident}_slice");
		syn::Ident::new(&slice_ident,ident.span())
	});
	let field_slice_mut_fn_idents=fields.named.iter().map(|field|{
		let ident=field.ident.as_ref().unwrap();
		let slice_ident=format!("{ident}_slice_mut");
		syn::Ident::new(&slice_ident,ident.span())
	});
	let field_access = quote! {
		impl<A: ::colvec::alloc::Allocator> #colvec_ident<A>{
			#(
				#[inline]
				pub const fn #field_slice_fn_idents(&self) -> &[#field_types] {
					unsafe {
						::core::slice::from_raw_parts(
							self.as_ptr()
								.add(self.buf.capacity() * <#ident as ::colvec::raw::StructInfo<#fields_count>>::FIELDS.offset_of(#field_indices))
								.cast::<#field_types>(),
							self.len
						)
					}
				}
				#[inline]
				pub const fn #field_slice_mut_fn_idents(&mut self) -> &mut [#field_types] {
					unsafe {
						::core::slice::from_raw_parts_mut(
							self.as_mut_ptr()
								.add(self.buf.capacity() * <#ident as ::colvec::raw::StructInfo<#fields_count>>::FIELDS.offset_of(#field_indices))
								.cast::<#field_types>(),
							self.len
						)
					}
				}
			)*
		}
	};

	quote! {
		#colvec
		#global

		#struct_info

		#impls
		#field_access
	}.into()
}

#[cfg(test)]
mod tests {
	use super::*;
	use syn::parse_quote;

	#[test]
	fn snapshot_test1() {
		let test1:syn::ItemStruct = parse_quote! {
			pub struct Test{
				field0:u8,
				field1:Option<u8>,
				field2:i16,
				field3:u32,
			}
		};

		let output = colvec_derive_inner(test1.into());

		// pretend it outputs a file
		let as_file = syn::parse_file(&output.to_string()).unwrap();

		// format it in a pretty way
		let formatted = prettyplease::unparse(&as_file);

		// snapshot-test it
		insta::assert_snapshot!(formatted);
	}
}
