use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
  Attribute, DeriveInput, GenericParam, Ident, Lifetime, Visibility, parse_macro_input, parse_quote, token::Pub, visit::Visit
};

fn find_crate_name(attrs: &[Attribute]) -> syn::Result<TokenStream> {
  attrs
    .iter()
    .find_map(|attr| {
      if !attr.path().is_ident("aubystd_crate") {
        return None;
      }

      Some(
        attr.meta.require_list().map(|meta| meta.tokens.clone()).map(|crate_name| {
          if crate_name.to_string() != "crate" {
            quote!(::#crate_name)
          } else {
            crate_name
          }
        }),
      )
    })
    .unwrap_or_else(|| {
      Ok(match proc_macro_crate::crate_name("aubystd").unwrap() {
        proc_macro_crate::FoundCrate::Itself => quote!(crate),
        proc_macro_crate::FoundCrate::Name(name) => {
          let name = Ident::new(&name, Span::call_site());
          quote!(::#name)
        }
      })
    })
}

#[proc_macro_derive(SliceDst, attributes(aubystd_crate))]
pub fn slice_dst_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let input = parse_macro_input!(item as DeriveInput);

  fn slice_dst_derive(
    DeriveInput {
      attrs,
      vis: _,
      ident,
      mut generics,
      data,
    }: DeriveInput,
  ) -> syn::Result<TokenStream> {
    let crate_name = find_crate_name(&attrs)?;

    let syn::Data::Struct(data_struct) = data else {
      return Err(syn::Error::new(ident.span(), "only structs are supported"));
    };

    enum FieldsKind {
      Named,
      Unnamed,
    }

    let (last, last_index, fields, fields_kind) = match data_struct.fields {
      syn::Fields::Named(mut fields_named) => (
        fields_named.named.pop().map(|last| last.into_value()),
        fields_named.named.len(),
        fields_named.named.into_iter(),
        FieldsKind::Named,
      ),
      syn::Fields::Unnamed(mut fields_unnamed) => (
        fields_unnamed.unnamed.pop().map(|last| last.into_value()),
        fields_unnamed.unnamed.len(),
        fields_unnamed.unnamed.into_iter(),
        FieldsKind::Unnamed,
      ),
      syn::Fields::Unit => return Err(syn::Error::new(ident.span(), "unit structs cannot be slice DSTs")),
    };
    let fields = fields.map(|mut field| {
      field.vis = Visibility::Public(Pub(Span::call_site()));
      field
    });

    let Some(last) = last else {
      return Err(syn::Error::new(
        ident.span(),
        "struct must contain at least one unsized field",
      ));
    };

    struct TypeParamVisitor<'ast> {
      type_param_ident: &'ast Ident,
      used_type_params: Vec<Ident>,
    }

    impl<'ast> Visit<'ast> for TypeParamVisitor<'ast> {
      fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        if self.type_param_ident == ident {
          self.used_type_params.push(ident.clone());
        }
      }
    }

    if let Some(used_type_params) = generics.type_params().find_map(|param| {
      let mut visitor = TypeParamVisitor {
        type_param_ident: &param.ident,
        used_type_params: vec![],
      };
      visitor.visit_type_param(param);
      (visitor.used_type_params.len() > 0).then_some(visitor.used_type_params)
    }) {
      // we have generic params in the last field's type
      for param_type in used_type_params {
        for ele in generics.type_params_mut() {
          if ele.ident == param_type {
            ele.bounds.push(parse_quote!(#crate_name::alloc::SliceDst));
            break;
          }
        }
      }
    }

    let last_ty = last.ty;

    let header_name = match fields.len() {
      2.. => quote!(Header),
      1 => fields.clone().next().unwrap().ty.to_token_stream(),
      0 => quote!(()),
    };

    let header_assoc_generics = (fields.len() > 1 && generics.params.len() > 0).then_some({
      let params = generics.params.iter().map(|param| match param {
        syn::GenericParam::Lifetime(lifetime_param) => lifetime_param.lifetime.to_token_stream(),
        syn::GenericParam::Type(type_param) => type_param.ident.to_token_stream(),
        syn::GenericParam::Const(const_param) => const_param.ident.to_token_stream(),
      });
      quote!(<#(#params),*>)
    });

    let last_ident = match fields_kind {
      FieldsKind::Named => last.ident.to_token_stream(),
      FieldsKind::Unnamed => last_index.to_token_stream(),
    };

    let addr_of_slice = if fields.len() < 2 {
      quote! {
        unsafe { &raw mut (*ptr).#last_ident }
      }
    } else {
      quote! {
        <#last_ty as #crate_name::alloc::SliceDst>::addr_of_slice(unsafe { &raw mut (*ptr).#last_ident })
      }
    };

    let header = if fields.len() < 2 {
      quote!()
    } else {
      let mut generics = generics.clone();

      struct UnusedGenerics {
        used_params: Vec<Ident>,
        used_lifetimes: Vec<Lifetime>,
      }

      impl<'ast> Visit<'ast> for UnusedGenerics {
        fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
          self.used_lifetimes.push(lifetime.clone());
        }
        fn visit_ident(&mut self, i: &'ast proc_macro2::Ident) {
          self.used_params.push(i.clone());
        }
      }

      let fields_unused = fields.clone();
      let mut unused_generics = UnusedGenerics {
        used_lifetimes: vec![],
        used_params: vec![],
      };
      for ele in fields_unused {
        unused_generics.visit_field(&ele);
      }
      unused_generics.visit_type(&last_ty);

      generics.params = generics
        .params
        .into_iter()
        .filter(|param| match param {
          GenericParam::Lifetime(lifetime) => unused_generics.used_lifetimes.contains(&lifetime.lifetime),
          GenericParam::Const(const_param) => unused_generics.used_params.contains(&const_param.ident),
          GenericParam::Type(type_param) => unused_generics.used_params.contains(&type_param.ident),
        })
        .collect();

      match fields_kind {
        FieldsKind::Named => {
          let last_ident_header = Ident::new(&format!("{last_ident}_header"), Span::call_site());
          quote! {
            struct #header_name #generics {
              #(#fields,)*
              pub #last_ident_header: <#last_ty as #crate_name::alloc::SliceDst>::Header
            }
          }
        }
        FieldsKind::Unnamed => {
          quote! {
            struct #header_name #generics (#(#fields,)* <#last_ty as #crate_name::alloc::SliceDst>::Header);
          }
        }
      }
    };

    let generics_str = generics.to_token_stream().to_string();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
      const _: () = {
        let _ = #generics_str;
        #header

        impl #impl_generics #crate_name::alloc::SliceDst for #ident #ty_generics #where_clause {
          type Header = #header_name #header_assoc_generics;
          type Element = <#last_ty as #crate_name::alloc::SliceDst>::Element;

          fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element] {
            #addr_of_slice
          }
        }
      };

    })
  }

  match slice_dst_derive(input) {
    Ok(output) => output.into(),
    Err(error) => error.into_compile_error().into(),
  }
}
