use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input, spanned::Spanned};

#[proc_macro_derive(Dst)]
pub fn dst_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let input = parse_macro_input!(item as DeriveInput);

  fn dst_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let syn::Data::Struct(data_struct) = input.data else {
      return Err(syn::Error::new(input.span(), "only structs are supported"));
    };

    let syn::Fields::Named(fields) = data_struct.fields else {
      return Err(syn::Error::new(
        data_struct.fields.span(),
        "only structs with named fields are supported",
      ));
    };

    // let Some(last) = fields.named.last() else {
    //   return Err(syn::Error::new(
    //     fields.span(),
    //     "struct must contain at least one unsized field",
    //   ));
    // };

    Ok(quote! {
      struct Prefix
    })
  }

  match dst_derive(input) {
    Ok(output) => output.into(),
    Err(error) => error.into_compile_error().into(),
  }
}
