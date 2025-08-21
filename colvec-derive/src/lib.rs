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
	let fields = quote! {
		const FIELDS: ::colvec::fields::Fields<#fields_count> = ::colvec::fields::Fields::from_sizes([
			size_of::<u8>(),
			size_of::<i32>(),
		]);
	};

	// this trait smuggles information about the input type into RawColVec and RawColVecInner
	let smuggle_outer = quote! {
		impl ::colvec::raw::SmuggleOuter for #ident{
			const LAYOUT: ::core::alloc::Layout = unsafe {
				let size = size_of::<u8>() + size_of::<i32>();
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

	quote! {
		#colvec
		#global

		#fields
		#smuggle_outer

	}.into()
}
