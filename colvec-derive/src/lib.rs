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

	quote! {
		#colvec
	}.into()
}
