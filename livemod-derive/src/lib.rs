use proc_macro2::{Delimiter, Ident, Spacing, TokenStream, TokenTree};
use quote::quote;
use syn::{parenthesized, parse::Parse, punctuated::Punctuated, DeriveInput, LitStr, Token};

#[proc_macro_derive(LiveMod, attributes(livemod))]
pub fn livemod_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => {
            match st.fields {
                syn::Fields::Named(fields) => {
                    let struct_name = ast.ident;
                    let (fields, matches) = fields
                        .named
                        .into_iter()
                        .filter_map(|field| {
                            let attrs = field
                                .attrs
                                .into_iter()
                                .filter_map(|attr| {
                                    if attr.path.is_ident("livemod") {
                                        Some(syn::parse2(attr.tokens).unwrap())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>();
                            if !attrs.iter().any(|attr| matches!(attr, Attr::Skip)) {
                                //TODO: #[livemod(repr)]
                                let ident = field.ident.unwrap();
                                let mut name = attrs
                                    .iter()
                                    .filter_map(|attr| match attr {
                                        Attr::Rename(name) => Some(name.clone()),
                                        _ => None,
                                    })
                                    .next_back()
                                    .unwrap_or(ident.to_string());
                                let name =
                                    if !attrs.iter().any(|attr| matches!(attr, Attr::PreserveCase))
                                    {
                                        name.as_mut_str()[..1].make_ascii_uppercase();
                                        name
                                    } else {
                                        ident.to_string()
                                    };
                                Some((
                                    quote! {
                                        ::livemod::TrackedData {
                                            name: String::from(#name),
                                            data_type: ::livemod::LiveMod::data_type(&self.#ident)
                                        }
                                    },
                                    quote! {
                                        #name => &mut self.#ident
                                    },
                                ))
                            } else {
                                None
                            }
                        })
                        .unzip::<_, _, Vec<_>, Vec<_>>();
                    let gen = quote! {
                        #[automatically_derived]
                        impl ::livemod::LiveMod for #struct_name {
                            fn data_type(&self) -> ::livemod::TrackedDataRepr {
                                ::livemod::TrackedDataRepr::Struct {
                                    name: String::from(stringify!(#struct_name)),
                                    fields: vec![
                                        #(#fields),*
                                    ]
                                }
                            }

                            fn get_named_value(&mut self, name: &str) -> &mut ::livemod::LiveMod {
                                match name {
                                    #(#matches ,)*
                                    _ => panic!("Unexpected value name!"),
                                }
                            }

                            fn set_self(&mut self, value: ::livemod::TrackedDataValue) {
                                panic!("Unexpected set operation!")
                            }
                        }
                    };
                    gen.into()
                }
                syn::Fields::Unnamed(fields) => todo!(),
                syn::Fields::Unit => todo!(),
            }
        }
        syn::Data::Enum(en) => todo!(),
        syn::Data::Union(_) => todo!(),
    }
}

enum Attr {
    Skip,
    Rename(String),
    PreserveCase,
    Repr(Ident, Punctuated<TokenStream, Token![,]>),
}

impl Parse for Attr {
    fn parse(direct_input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input;
        parenthesized!(input in direct_input);
        let attr_type: Ident = input.parse()?;
        if attr_type == "skip" {
            if !input.is_empty() {
                Err(input.error("Expected end of attribute content"))?;
            }
            Ok(Attr::Skip)
        } else if attr_type == "preserve_case" {
            if !input.is_empty() {
                Err(input.error("Expected end of attribute content"))?;
            }
            Ok(Attr::PreserveCase)
        } else if attr_type == "rename" {
            input.parse::<Token![=]>()?;
            let new_name: LitStr = input.parse()?;
            Ok(Attr::Rename(new_name.value()))
        } else if attr_type == "repr" {
            input.parse::<Token![=]>()?;
            let trait_name = input.parse()?;
            let arguments;
            parenthesized!(arguments in input);
            Ok(Attr::Repr(
                trait_name,
                arguments.parse_terminated(TokenStream::parse)?,
            ))
        } else {
            Err(input.error("Unknown attribute content"))
        }
    }
}
