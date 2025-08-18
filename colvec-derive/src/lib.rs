use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input,DeriveInput};

#[proc_macro_derive(ColVec)]
pub fn colvec_derive(input:TokenStream)->TokenStream{
	let input: DeriveInput=parse_macro_input!(input);

	let ident=input.ident;

	let ident_str={
		let mut ident=ident.to_string();
		ident.push_str("ColVec");
		ident
	};

	let ident=syn::Ident::new(&ident_str,ident.span());

	// colvec::colvec!();
	let props:[syn::Field;0]=[];

	let mut colvec=syn::ItemStruct{
		attrs: Vec::new(),
		vis: syn::Visibility::Public(syn::token::Pub::default()),
		struct_token: syn::token::Struct::default(),
		ident,
		generics: input.generics.into(),
		fields: syn::Fields::Named(syn::FieldsNamed {
			brace_token: syn::token::Brace::default(),
			named: props.into_iter().collect(),
		}),
		semi_token: None,
	};

	colvec.into_token_stream().into()
}
