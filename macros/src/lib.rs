use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
  DeriveInput, GenericParam, Ident, Lifetime, Token, Type, parse::{Parse, ParseStream, Parser}, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, visit::Visit
};

type MetaPunct = Punctuated<Metadata, Token![,]>;
fn find_meta_kv(meta: &MetaPunct, key: &str) -> syn::Result<Option<(Span, TokenTree)>> {
  meta
    .iter()
    .find_map(|attr| {
      if attr.key() == key {
        return if let Metadata::KeyValue { span, value, .. } = attr {
          Some(Ok((span.clone(), value.clone())))
        } else {
          Some(Err(syn::Error::new(attr.key().span(), "expected key value pair")))
        };
      }
      None
    })
    .transpose()
}
fn find_meta_list(meta: &MetaPunct, key: &str) -> syn::Result<Option<(Span, Punctuated<TokenTree, Token![,]>)>> {
  meta
    .iter()
    .find_map(|attr| {
      if attr.key() == key {
        return if let Metadata::List { span, values, .. } = attr {
          Some(Ok((span.clone(), values.clone())))
        } else {
          Some(Err(syn::Error::new(attr.key().span(), "expected list")))
        };
      }
      None
    })
    .transpose()
}

fn find_crate_name(meta: &MetaPunct) -> syn::Result<TokenStream> {
  Ok(
    find_meta_kv(meta, "crate")?.map(|(_, value)| value.into_token_stream()).unwrap_or_else(|| {
      match proc_macro_crate::crate_name("aubystd").unwrap() {
        proc_macro_crate::FoundCrate::Itself => quote!(crate),
        proc_macro_crate::FoundCrate::Name(name) => {
          let name = Ident::new(&name, Span::call_site());
          quote!(::#name)
        }
      }
    }),
  )
}

enum Metadata {
  KeyValue {
    span: Span,
    key: Ident,
    value: TokenTree,
  },
  List {
    span: Span,
    key: Ident,
    values: Punctuated<TokenTree, Token![,]>,
  },
}
impl Metadata {
  pub fn key(&self) -> &Ident {
    match self {
      Metadata::KeyValue { key, .. } => key,
      Metadata::List { key, .. } => key,
    }
  }
}
impl Parse for Metadata {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let key: Ident = input.parse()?;
    if input.peek(Token![=]) {
      let _: Token![=] = input.parse()?;
      let value: TokenTree = input.parse()?;
      let span = Span::join(&key.span(), value.span()).unwrap();
      Ok(Metadata::KeyValue { span, key, value })
    } else if let Ok(list) = input.step(|c| {
      let x = c.group(Delimiter::Parenthesis).ok_or(syn::Error::new(Span::call_site(), "parens"))?;
      Ok((x.0.token_stream(), x.2))
    }) {
      let values = Punctuated::parse_terminated.parse2(list)?;
      let span = Span::join(&key.span(), values.span()).unwrap();
      Ok(Metadata::List { span, key, values })
    } else {
      Err(syn::Error::new(
        input.span(),
        format!("invalid attribute argument \"{key}\""),
      ))
    }
  }
}

#[proc_macro_attribute]
pub fn slice_dst(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
  fn slice_dst_derive(
    attr: TokenStream,
    item: TokenStream,
    DeriveInput {
      attrs,
      vis,
      ident,
      mut generics,
      data,
    }: DeriveInput,
  ) -> syn::Result<TokenStream> {
    let meta: Punctuated<Metadata, Token![,]> = if attr.is_empty() {
      Punctuated::new()
    } else {
      Punctuated::parse_separated_nonempty.parse2(attr)?
    };
    let _: () = meta
      .iter()
      .filter_map(|x| {
        ["header", "derive", "crate"]
          .iter()
          .all(|key| *x.key() != *key)
          .then(|| syn::Error::new(x.key().span(), format!("invalid argument: {}", x.key())))
      })
      .reduce(|mut left, right| {
        left.combine(right);
        left
      })
      .map(|err| Err(err))
      .transpose()?
      .unwrap_or_default();

    let crate_name = find_crate_name(&meta)?;

    let attrs: Vec<_> = attrs.into_iter().filter(|attr| !attr.path().is_ident("derive")).collect();
    let derive = find_meta_list(&meta, "derive")?.unwrap_or((Span::call_site(), parse_quote!()));

    let syn::Data::Struct(data_struct) = data else {
      return Err(syn::Error::new(ident.span(), "only structs are supported"));
    };

    let reprs = attrs
      .iter()
      .filter_map(|attr| {
        if !attr.path().is_ident("repr") {
          return None;
        }

        let mut repr = None;
        let result = attr
          .parse_nested_meta(|meta| {
            if ["C", "transparent", "align", "packed"].iter().any(|ident| meta.path.is_ident(ident)) {
              repr = Some(meta.path);
              Ok(())
            } else {
              Err(syn::Error::new(meta.path.span(), "unsupported repr"))
            }
          })
          .map(|_| repr.unwrap());
        Some(result)
      })
      .collect::<syn::Result<Vec<_>>>()?;

    if reprs.len() == 0 || reprs.iter().all(|path| !path.is_ident("C") && !path.is_ident("transparent")) {
      return Err(syn::Error::new(
        ident.span(),
        "struct must be either repr(C) or repr(transparent)",
      ));
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
    // let fields = fields.map(|mut field| {
    //   field.vis = Visibility::Public(Pub(Span::call_site()));
    //   field
    // });

    let Some(last) = last else {
      return Err(syn::Error::new(
        ident.span(),
        "struct must contain at least one unsized field",
      ));
    };

    let last_ty = last.ty;

    let header_name = match fields.len() {
      1.. => {
        // Ident::new(&format!("{ident}Header"), Span::call_site()).to_token_stream()

        let (_, header) =
          find_meta_kv(&meta, "header")?.ok_or(syn::Error::new(Span::call_site(), "must have a header name"))?;
        header.to_token_stream()
      }
      // 1 => fields.clone().next().unwrap().ty.to_token_stream(),
      0 => quote!(()),
    };

    let header_assoc_generics = (generics.params.len() > 0).then_some({
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

    let addr_of_slice = if fields.len() < 1 {
      quote! {
        unsafe { &raw mut (*ptr).#last_ident }
      }
    } else {
      quote! {
        <#last_ty as #crate_name::alloc::SliceDst>::addr_of_slice(unsafe { &raw mut (*ptr).#last_ident })
      }
    };

    let header = if fields.len() < 1 {
      quote!()
    } else {
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
      // return Err(syn::Error::new(Span::call_site(), format!("fuck you {last_ty:?}")));
      unused_generics.visit_type(&last_ty);
      if !matches!(last_ty, Type::Slice(_)) && unused_generics.used_params.len() == 1 {
        for param_name in &unused_generics.used_params {
          if let Some(param) = generics.type_params_mut().find(|param| param.ident == *param_name) {
            param.bounds.push(parse_quote!(#crate_name::alloc::SliceDst));
          }
        }
      };
      for ele in fields_unused {
        unused_generics.visit_field(&ele);
      }

      generics.params = generics
        .params
        .into_iter()
        .filter(|param| match param {
          GenericParam::Lifetime(lifetime) => unused_generics.used_lifetimes.contains(&lifetime.lifetime),
          GenericParam::Const(const_param) => unused_generics.used_params.contains(&const_param.ident),
          GenericParam::Type(type_param) => unused_generics.used_params.contains(&type_param.ident),
        })
        .collect();

      let (derive_span, derives) = derive;
      let derives = derives.iter();
      let derive = quote_spanned! {derive_span=>#[derive(#(#derives),*)]};
      match fields_kind {
        FieldsKind::Named => {
          let last_ident_header = Ident::new(&format!("{last_ident}_header"), Span::call_site());
          quote! {
            #[doc(hidden)]
            #derive
            #(#attrs)*
            #vis struct #header_name #generics {
              #(#fields,)*
              pub #last_ident_header: <#last_ty as #crate_name::alloc::SliceDst>::Header
            }
          }
        }
        FieldsKind::Unnamed => {
          quote! {
            #[doc(hidden)]
            #derive
            #(#attrs)*
            #vis struct #header_name #generics (#(#fields,)* <#last_ty as #crate_name::alloc::SliceDst>::Header);
          }
        }
      }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
      #item
      #header

      unsafe impl #impl_generics #crate_name::alloc::SliceDst for #ident #ty_generics #where_clause {
        type Header = #header_name #header_assoc_generics;
        type Element = <#last_ty as #crate_name::alloc::SliceDst>::Element;

        fn addr_of_slice(ptr: *mut Self) -> *mut [Self::Element] {
          #addr_of_slice
        }
      }
    })
  }

  let input_item = item.clone();
  let input = parse_macro_input!(input_item as DeriveInput);
  match slice_dst_derive(attr.into(), item.into(), input) {
    Ok(output) => output.into(),
    Err(error) => error.into_compile_error().into(),
  }
}

/// For things that need to be named, but don't have a nice name yet
#[proc_macro_attribute]
pub fn aubystd_bikeshed_name(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
  item
}
