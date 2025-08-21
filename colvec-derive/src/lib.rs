use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input,DeriveInput};

#[proc_macro_derive(ColVec)]
pub fn colvec_derive(input:TokenStream)->TokenStream{
	let input:DeriveInput=parse_macro_input!(input);

	match input.data{
		syn::Data::Struct(syn::DataStruct{fields:syn::Fields::Named(fields_named),..})=>derive_struct(input.ident,input.vis,fields_named),
		_=>unimplemented!("Only structs are supported"),
	}
}

fn derive_struct(ident:syn::Ident,vis:syn::Visibility,fields:syn::FieldsNamed)->TokenStream{
	let colvec_ident_string=format!("{ident}ColVec");
	let colvec_ident=syn::Ident::new(&colvec_ident_string,ident.span());

	let colvec = quote!{
		#vis struct #colvec_ident<A: ::colvec::alloc::Allocator = ::colvec::alloc::Global>{
			buf: ::colvec::raw::RawColVec<#ident, A>,
			len: usize,
		}
	};

	let global = quote! {
		impl #colvec_ident<::colvec::alloc::Global>{
			#[inline]
			#[must_use]
			pub const fn new() -> Self {
				Self { buf: ::colvec::raw::RawColVec::new(), len: 0 }
			}
		}
	};

	// TODO: dont make global constants
	let fields_count=fields.named.len();
	let fields_types=fields.named.iter().map(|field|field.ty.clone());
	let fields_global = quote! {
		const FIELDS: ::colvec::fields::Fields<#fields_count> = ::colvec::fields::Fields::from_sizes([
			#(size_of::<#fields_types>()),*
		]);
	};

	// this trait smuggles information about the input type into RawColVec and RawColVecInner
	let fields_types=fields.named.iter().map(|field|field.ty.clone());
	let smuggle_outer = quote! {
		impl ::colvec::raw::SmuggleOuter for #ident{
			const LAYOUT: ::core::alloc::Layout = unsafe {
				let size = #(size_of::<#fields_types>())+*;
				let align = align_of::<#ident>();
				::core::alloc::Layout::from_size_align_unchecked(size, align)
			};
			unsafe fn move_fields(
				ptr: *mut u8,
				old_capacity: usize,
				new_capacity: usize,
				len: usize,
			) {
				unsafe { FIELDS.move_fields(ptr, old_capacity, new_capacity, len) }
			}
		}
	};

	let field_indices=0..fields.named.len();
	let field_types=fields.named.iter().map(|field|field.ty.clone());
	let field_idents=fields.named.iter().map(|field|field.ident.as_ref().unwrap().clone());
	let impls = quote! {
		impl<A: ::colvec::alloc::Allocator> #colvec_ident<A>{
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
							.add(self.buf.capacity() * FIELDS.offset_of(#field_indices))
							.cast::<#field_types>()
							.add(len);
						::core::ptr::write(end, value.#field_idents);
					)*
				}
				self.len = len + 1;
			}
		}
	};

	let field_indices=0..fields.named.len();
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
				pub const fn #field_slice_fn_idents(&self) -> &[#field_types] {
					unsafe {
						core::slice::from_raw_parts(
							self.as_ptr()
								.add(self.buf.capacity() * FIELDS.offset_of(#field_indices))
								.cast::<#field_types>(),
							self.len
						)
					}
				}
				pub const fn #field_slice_mut_fn_idents(&mut self) -> &mut [#field_types] {
					unsafe {
						core::slice::from_raw_parts_mut(
							self.as_mut_ptr()
								.add(self.buf.capacity() * FIELDS.offset_of(#field_indices))
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

		#fields_global
		#smuggle_outer

		#impls
		#field_access
	}.into()
}
