use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::{DeriveInput, FieldsNamed, FieldsUnnamed, LitStr, Token, parenthesized, parse::Parse, punctuated::Punctuated};

#[proc_macro_derive(LiveMod, attributes(livemod))]
pub fn livemod_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => match st.fields {
            syn::Fields::Named(fields) => {
                named_struct(ast.ident, fields).into()
            }
            syn::Fields::Unnamed(fields) => {
                tuple_struct(ast.ident, fields).into()
            },
            syn::Fields::Unit => {
                let gen = quote! {
                    compile_error!("Derive not supported on unit struct")
                };
                gen.into()
            },
        },
        syn::Data::Enum(_en) => todo!(),
        syn::Data::Union(_) => {
            let gen = quote! {
                compile_error!("Derive not supported on union")
            };
            gen.into()
        },
    }
}

fn named_struct(struct_name: Ident, fields: FieldsNamed) -> TokenStream {
    let (fields, matches_gets) = fields
        .named
        .into_iter()
        .filter_map(|field| {
            let attrs = match field
                .attrs
                .into_iter()
                .filter_map(|attr| {
                    if attr.path.is_ident("livemod") {
                        Some(syn::parse2(attr.tokens))
                    } else {
                        None
                    }
                })
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(attrs) => attrs,
                Err(error) => {
                    let ident = field.ident.unwrap();
                    let name = ident.to_string();
                    return Some((
                        error.to_compile_error(),
                        (quote! { #name => &mut self.#ident }, quote! { (#name.to_owned(), ::livemod::LiveMod::get_self(&self.#ident)) }),
                    ));
                }
            };
            if !attrs.iter().any(|attr| matches!(attr, Attr::Skip)) {
                let ident = field.ident.unwrap();
                let name = attrs
                    .iter()
                    .filter_map(|attr| match attr {
                        Attr::Rename(name) => Some(name.clone()),
                        _ => None,
                    })
                    .next_back()
                    .unwrap_or({
                        let mut name = ident.to_string();
                        name.as_mut_str()[..1].make_ascii_uppercase();
                        name = name.replace('_', " ");
                        name
                    });
                let repr = if let Some(Attr::Repr(trait_, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    let mut repr_method = format!("repr_{}", trait_,);
                    repr_method.make_ascii_lowercase();
                    let repr_method = Ident::new(&repr_method, trait_.span());
                    quote! { #trait_::#repr_method(&self.#ident, #args) }
                } else {
                    quote! { ::livemod::LiveMod::repr_default(&self.#ident) }
                };
                Some((
                    quote! {
                        ::livemod::TrackedData {
                            name: String::from(#name),
                            data_type: #repr,
                            triggers: vec![],
                        }
                    },
                    (
                        quote! {
                            #name => &mut self.#ident
                        },
                        quote! {
                            (#name.to_owned(), ::livemod::LiveMod::get_self(&self.#ident))
                        }
                    )
                ))
            } else {
                None
            }
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (matches, gets) = matches_gets.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

    quote! {
        #[automatically_derived]
        impl ::livemod::LiveMod for #struct_name {
            fn repr_default(&self) -> ::livemod::TrackedDataRepr {
                ::livemod::TrackedDataRepr::Struct {
                    name: String::from(stringify!(#struct_name)),
                    fields: vec![
                        #(#fields),*
                    ],
                    triggers: vec![]
                }
            }

            fn get_named_value(&mut self, name: &str) -> &mut ::livemod::LiveMod {
                match name {
                    #(#matches ,)*
                    _ => panic!("Unexpected value name!"),
                }
            }

            fn trigger(&mut self, trigger: ::livemod::Trigger) -> bool {
                panic!("Unexpected trigger operation!")
            }

            fn get_self(&self) -> ::livemod::TrackedDataValue {
                ::livemod::TrackedDataValue::Struct(vec![
                    #(#gets),*
                ])
            }
        }
    }
}

fn tuple_struct(struct_name: Ident, fields: FieldsUnnamed) -> TokenStream {
    let (fields, matches_gets) = fields
        .unnamed
        .into_iter()
        .enumerate()
        .filter_map(|(i, field)| {
            let attrs = match field
                .attrs
                .into_iter()
                .filter_map(|attr| {
                    if attr.path.is_ident("livemod") {
                        Some(syn::parse2(attr.tokens))
                    } else {
                        None
                    }
                })
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(attrs) => attrs,
                Err(error) => {
                    let name = i.to_string();
                    let ident = Literal::usize_unsuffixed(i); //TODO: Set span
                    return Some((
                        error.to_compile_error(),
                        (quote! { #name => &mut self.#ident }, quote! { (#name.to_owned(), ::livemod::LiveMod::get_self(&self.#ident)) }),
                    ));
                }
            };
            if !attrs.iter().any(|attr| matches!(attr, Attr::Skip)) {
                let name = i.to_string();
                let ident = Literal::usize_unsuffixed(i); //TODO: Set span
                let repr = if let Some(Attr::Repr(trait_, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    let mut repr_method = format!("repr_{}", trait_,);
                    repr_method.make_ascii_lowercase();
                    let repr_method = Ident::new(&repr_method, trait_.span());
                    quote! { #trait_::#repr_method(&self.#ident, #args) }
                } else {
                    quote! { ::livemod::LiveMod::repr_default(&self.#ident) }
                };
                Some((
                    quote! {
                        ::livemod::TrackedData {
                            name: String::from(#name),
                            data_type: #repr,
                            triggers: vec![],
                        }
                    },
                    (
                        quote! {
                            #name => &mut self.#ident
                        },
                        quote! {
                            (#name.to_owned(), ::livemod::LiveMod::get_self(&self.#ident))
                        }
                    )
                ))
            } else {
                None
            }
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (matches, gets) = matches_gets.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

    quote! {
        #[automatically_derived]
        impl ::livemod::LiveMod for #struct_name {
            fn repr_default(&self) -> ::livemod::TrackedDataRepr {
                ::livemod::TrackedDataRepr::Struct {
                    name: String::from(stringify!(#struct_name)),
                    fields: vec![
                        #(#fields),*
                    ],
                    triggers: vec![]
                }
            }

            fn get_named_value(&mut self, name: &str) -> &mut ::livemod::LiveMod {
                match name {
                    #(#matches ,)*
                    _ => panic!("Unexpected value name!"),
                }
            }

            fn trigger(&mut self, trigger: ::livemod::Trigger) -> bool {
                panic!("Unexpected trigger operation!")
            }

            fn get_self(&self) -> ::livemod::TrackedDataValue {
                ::livemod::TrackedDataValue::Struct(vec![
                    #(#gets),*
                ])
            }
        }
    }
}

enum Attr {
    Skip,
    Rename(String),
    Repr(Ident, Punctuated<TokenStream, Token![,]>),
}

impl Parse for Attr {
    fn parse(direct_input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input;
        parenthesized!(input in direct_input);
        let attr_type: Ident = input.parse()?;
        if attr_type == "skip" {
            if !input.is_empty() {
                return Err(input.error("Expected end of attribute content"));
            }
            Ok(Attr::Skip)
        } else if attr_type == "rename" {
            input.parse::<Token![=]>()?;
            let new_name: LitStr = input.parse()?;
            Ok(Attr::Rename(new_name.value()))
        } else if attr_type == "repr" {
            input.parse::<Token![=]>()?;
            let trait_name = input.parse()?;
            if !input.is_empty() {
                let arguments;
                parenthesized!(arguments in input);
                Ok(Attr::Repr(
                    trait_name,
                    arguments.parse_terminated(TokenStream::parse)?,
                ))
            } else {
                Ok(Attr::Repr(trait_name, Punctuated::new()))
            }
        } else {
            Err(syn::Error::new(
                attr_type.span(),
                "Unrecognised attribute tag",
            ))
        }
    }
}
