use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use syn::{
    parenthesized, parse::Parse, punctuated::Punctuated, DataEnum, DeriveInput, FieldsNamed,
    FieldsUnnamed, LitStr, Token,
};

#[proc_macro_derive(LiveMod, attributes(livemod))]
pub fn livemod_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => {
            let struct_name = ast.ident;
            let (fields, matches, gets) = match st.fields {
                syn::Fields::Named(fields) => derive_fields_named(fields),
                syn::Fields::Unnamed(fields) => derive_fields_unnamed(fields),
                syn::Fields::Unit => {
                    let gen = quote! {
                        compile_error!("Derive not supported on unit struct")
                    };
                    return gen.into();
                }
            };
            let gen = quote! {
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
            };
            gen.into()
        }
        syn::Data::Enum(en) => derive_enum(ast.ident, en).into(),
        syn::Data::Union(_) => {
            let gen = quote! {
                compile_error!("Derive not supported on union")
            };
            gen.into()
        }
    }
}

fn derive_fields_named(
    fields: FieldsNamed,
) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {
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

    (fields, matches, gets)
}

fn derive_fields_unnamed(
    fields: FieldsUnnamed,
) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {
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

    (fields, matches, gets)
}

fn derive_enum(enum_name: Ident, en: DataEnum) -> TokenStream {
    let (variants_fields, matches_gets_defaults) = en
        .variants
        .into_iter()
        .map(|variant| {
            let ident = variant.ident;
            let qualified_ident = quote! { #enum_name::#ident };
            let stringified_ident = ident.to_string();

            let (var_fields, var_matches, var_gets, var_default) = match variant.fields {
                syn::Fields::Named(fields) => {
                    derive_enum_fields_named(ident, qualified_ident, fields)
                }
                syn::Fields::Unnamed(fields) => {
                    derive_enum_fields_unnamed(ident, qualified_ident, fields)
                }
                syn::Fields::Unit => (
                    quote! { #qualified_ident => vec![] },
                    quote! { #qualified_ident => panic!("Variant has no fields!") },
                    quote! {
                        #qualified_ident => ::livemod::TrackedDataValue::Enum {
                            variant: #stringified_ident.to_owned(),
                            fields: vec![]
                        }
                    },
                    quote! { #stringified_ident => #qualified_ident },
                ),
            };

            (
                (
                    quote! { #stringified_ident.to_owned() },
                    quote! { #var_fields },
                ),
                (
                    quote! { #var_matches },
                    (quote! { #var_gets }, quote! { #var_default }),
                ),
            )
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (variants, fields) = variants_fields.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
    let (matches, gets_defaults) = matches_gets_defaults
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let (gets, defaults) = gets_defaults.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

    quote! {
        #[automatically_derived]
        impl ::livemod::LiveMod for #enum_name {
            fn repr_default(&self) -> ::livemod::TrackedDataRepr {
                ::livemod::TrackedDataRepr::Enum {
                    name: String::from(stringify!(#enum_name)),
                    variants: vec![
                        #(#variants),*
                    ],
                    fields: match self {
                        #(#fields ,)*
                    },
                    triggers: vec![]
                }
            }

            fn get_named_value(&mut self, name: &str) -> &mut ::livemod::LiveMod {
                match self {
                    #(#matches ,)*
                }
            }

            fn trigger(&mut self, trigger: ::livemod::Trigger) -> bool {
                let variant_name = trigger.try_into_set().unwrap().try_into_enum_variant().unwrap();
                *self = match variant_name.as_str() {
                    #(#defaults ,)*
                    name => panic!("Unknown variant name: {}", name)
                };
                true
            }

            fn get_self(&self) -> ::livemod::TrackedDataValue {
                match self {
                    #(#gets ,)*
                }
            }
        }
    }
}

fn derive_enum_fields_named(
    ident: Ident,
    qualified_ident: TokenStream,
    fields: FieldsNamed,
) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let stringified_ident = ident.to_string();
    let (fields_idents, matches_gets_defaults) = fields
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
                        (error.to_compile_error(), quote! { vec![] }),
                        (
                            quote! { #name => &mut self.#ident },
                            (
                                quote! { (#name.to_owned(), ::livemod::LiveMod::get_self(&self.#ident)) },
                                quote! { #ident: Default::default() }
                            )
                        ),
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
                    quote! { #trait_::#repr_method(#ident, #args) }
                } else {
                    quote! { ::livemod::LiveMod::repr_default(#ident) }
                };
                Some((
                    (
                        quote! {
                            ::livemod::TrackedData {
                                name: String::from(#name),
                                data_type: #repr,
                                triggers: vec![],
                            }
                        },
                        quote! {
                            #ident
                        }
                    ),
                    (
                        quote! {
                            #name => #ident
                        },
                        (
                            quote! {
                                (#name.to_owned(), ::livemod::LiveMod::get_self(#ident))
                            },
                            quote! {
                                #ident: Default::default()
                            }
                        ),
                    )
                ))
            } else {
                //TODO: Skipped fields can't be initialised...
                None
            }
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (fields, idents) = fields_idents.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
    let (matches, gets_defaults) = matches_gets_defaults
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let (gets, defaults) = gets_defaults.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

    let match_pattern = quote! { #qualified_ident { #(#idents ,)* .. } };
    (
        quote! {
            #match_pattern => vec![#(#fields),*]
        },
        quote! {
            #match_pattern => match name {
                #(#matches ,)*
                _ => panic!("No field {} in {}", name, #stringified_ident)
            }
        },
        quote! {
            #match_pattern => ::livemod::TrackedDataValue::Enum {
                variant: #stringified_ident.to_owned(),
                fields: vec![
                    #(#gets),*
                ]
            }
        },
        quote! {
            #stringified_ident => #qualified_ident {
                #(#defaults),*
            }
        },
    )
}

fn derive_enum_fields_unnamed(
    ident: Ident,
    qualified_ident: TokenStream,
    fields: FieldsUnnamed,
) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let stringified_ident = ident.to_string();
    let (fields_indices, matches_gets_defaults) = fields
        .unnamed
        .into_iter()
        .enumerate()
        .filter_map(|(i, field)| {
            let name = i.to_string();
            let ident = Ident::new(&format!("__{}", name), Span::call_site()); //TODO: Set span
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
                    return Some((
                        (error.to_compile_error(), ident.clone()),
                        (
                            quote! { #name => &mut #ident },
                            (
                                quote! { (#name.to_owned(), ::livemod::LiveMod::get_self(&#ident)) },
                                quote! { Default::default() },
                            ),
                        ),
                    ));
                }
            };
            if !attrs.iter().any(|attr| matches!(attr, Attr::Skip)) {
                let repr = if let Some(Attr::Repr(trait_, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    let mut repr_method = format!("repr_{}", trait_,);
                    repr_method.make_ascii_lowercase();
                    let repr_method = Ident::new(&repr_method, trait_.span());
                    quote! { #trait_::#repr_method(#ident, #args) }
                } else {
                    quote! { ::livemod::LiveMod::repr_default(#ident) }
                };
                Some((
                    (
                        quote! {
                            ::livemod::TrackedData {
                                name: String::from(#name),
                                data_type: #repr,
                                triggers: vec![],
                            }
                        },
                        ident.clone(),
                    ),
                    (
                        quote! {
                            #name => #ident
                        },
                        (
                            quote! {
                                (#name.to_owned(), ::livemod::LiveMod::get_self(#ident))
                            },
                            quote! { Default::default() }
                        )
                    ),
                ))
            } else {
                //TODO: Skipped fields can't be initialised.
                None
            }
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (fields, indices) = fields_indices.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
    let (matches, gets_defaults) = matches_gets_defaults
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let (gets, defaults) = gets_defaults.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

    let mut binding_names = vec![];
    for (i, ident) in indices.into_iter().enumerate() {
        while i > binding_names.len() {
            binding_names.push(Ident::new("_", Span::call_site()));
        }
        binding_names.push(ident);
    }

    let match_pattern = quote! { #qualified_ident (#(#binding_names ,)* ..)};
    (
        quote! {
            #match_pattern => vec![#(#fields),*]
        },
        quote! {
            #match_pattern => match name {
                #(#matches ,)*
                _ => panic!("No field {} in {}", name, #stringified_ident)
            }
        },
        quote! {
            #match_pattern => ::livemod::TrackedDataValue::Enum {
                variant: #stringified_ident.to_owned(),
                fields: vec![
                    #(#gets),*
                ]
            }
        },
        quote! {
            #stringified_ident => #qualified_ident(#(#defaults),*)
        },
    )
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
