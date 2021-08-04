use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use syn::{DataEnum, DeriveInput, Field, FieldsNamed, FieldsUnnamed, LitStr, Token, parenthesized, parse::Parse, punctuated::Punctuated};

#[proc_macro_derive(LiveMod, attributes(livemod))]
pub fn livemod_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => {
            let struct_name = ast.ident;
            let (FieldsDerive { idents, default_values, representations, get_named_values, get_selves }, named) = match st.fields {
                syn::Fields::Named(fields) => (derive_fields_named(fields), true),
                syn::Fields::Unnamed(fields) => (derive_fields_unnamed(fields), false),
                syn::Fields::Unit => {
                    let gen = quote! {
                        compile_error!("Derive not supported on unit struct")
                    };
                    return gen.into();
                }
            };

            let self_pattern = if named {
                quote! { Self { #(#idents),* } }
            } else {
                quote! { Self ( #(#idents),* ) }
            };

            let gen = quote! {
                #[automatically_derived]
                impl ::livemod::LiveMod for #struct_name {
                    fn repr_default(&self) -> ::livemod::TrackedDataRepr {
                        let #self_pattern = self;
                        ::livemod::TrackedDataRepr::Struct {
                            name: String::from(stringify!(#struct_name)),
                            fields: vec![
                                #(#representations),*
                            ],
                            triggers: vec![]
                        }
                    }

                    fn get_named_value(&mut self, name: &str) -> &mut ::livemod::LiveMod {
                        let #self_pattern = self;
                        match name {
                            #(#get_named_values ,)*
                            _ => panic!("Unexpected value name!"),
                        }
                    }

                    fn trigger(&mut self, trigger: ::livemod::Trigger) -> bool {
                        panic!("Unexpected trigger operation!")
                    }

                    fn get_self(&self) -> ::livemod::TrackedDataValue {
                        let #self_pattern = self;
                        ::livemod::TrackedDataValue::Struct(vec![
                            #(#get_selves),*
                        ])
                    }
                }
            };
            gen.into()
        }
        syn::Data::Enum(en) => todo!(),
        syn::Data::Union(_) => {
            let gen = quote! {
                compile_error!("Derive not supported on union")
            };
            gen.into()
        }
    }
}

struct FieldsDerive {
    idents: Vec<Ident>,
    default_values: Vec<TokenStream>,
    representations: Vec<TokenStream>,
    get_named_values: Vec<TokenStream>,
    get_selves: Vec<TokenStream>,
}

struct FieldDerive {
    ident: Ident,
    default_value: TokenStream,
    representation: Option<TokenStream>,
    get_named_value: Option<TokenStream>,
    get_self: Option<TokenStream>,
}

fn derive_fields_named(fields: FieldsNamed) -> FieldsDerive {
    let iter = fields.named.into_iter().map(|field| {
        let ident = field.ident.clone().unwrap();
        let name = ident.to_string();
        derive_field(ident, name, field)
    });

    let mut gen = FieldsDerive {
        idents: Vec::new(),
        default_values: Vec::new(),
        representations: Vec::new(),
        get_named_values: Vec::new(),
        get_selves: Vec::new(),
    };

    for field in iter {
        gen.idents.push(field.ident);
        gen.default_values.push(field.default_value);
        gen.representations.extend(field.representation);
        gen.get_named_values.extend(field.get_named_value);
        gen.get_selves.extend(field.get_self);
    }

    gen
}

fn derive_fields_unnamed(fields: FieldsUnnamed) -> FieldsDerive {
    let iter = fields.unnamed.into_iter().enumerate().map(|(i, field)| {
        let ident = Ident::new(&format!("__{}", i), Span::call_site());
        let name = i.to_string();
        derive_field(ident, name, field)
    });

    let mut gen = FieldsDerive {
        idents: Vec::new(),
        default_values: Vec::new(),
        representations: Vec::new(),
        get_named_values: Vec::new(),
        get_selves: Vec::new(),
    };

    for field in iter {
        gen.idents.push(field.ident);
        gen.default_values.push(field.default_value);
        gen.representations.extend(field.representation);
        gen.get_named_values.extend(field.get_named_value);
        gen.get_selves.extend(field.get_self);
    }

    gen
}

fn derive_field(ident: Ident, name: String, field: Field) -> FieldDerive {
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
            return FieldDerive {
                ident,
                default_value: error.to_compile_error(),
                representation: None,
                get_named_value: None,
                get_self: None,
            };
        }
    };

    let default_value = if let Some(default) = attrs.iter().find_map(|attr| match attr { Attr::Default(ts) => Some(ts), _ => None }) {
        default.clone()
    } else {
        quote! { ::std::default::Default::default() }
    };

    let (representation, get_named_value, get_self) = if attrs.iter().any(|attr| matches!(attr, Attr::Skip)) {
        (None, None, None)
    } else {
        let default_repr = quote! { ::livemod::DefaultRepr };
        let repr_struct = attrs.iter().find_map(|attr| match attr { Attr::Repr(ts) => Some(ts), _ => None }).unwrap_or(&default_repr);
        let representation = quote! {
            ::livemod::TrackedData {
                name: #name.to_owned(),
                data_type: ::livemod::LiveModRepr::repr(&#repr_struct, #ident) ,
                triggers: vec![]
            }
        };

        let get_named_value = quote! { #name => #ident };
        let get_self = quote! { (#name.to_owned(), ::livemod::LiveMod::get_self(#ident)) };
        (Some(representation), Some(get_named_value), Some(get_self))
    };

    FieldDerive {
        ident,
        default_value,
        representation,
        get_named_value,
        get_self,
    }
}

/*
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
                let field_name = field.ident.unwrap();
                let name = attrs
                    .iter()
                    .filter_map(|attr| match attr {
                        Attr::Rename(name) => Some(name.clone()),
                        _ => None,
                    })
                    .next_back()
                    .unwrap_or({
                        let mut name = field_name.to_string();
                        name.as_mut_str()[..1].make_ascii_uppercase();
                        name = name.replace('_', " ");
                        name
                    });
                let field_type = field.ty;
                let repr = if let Some(Attr::Repr(struct_name, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    quote! { <_ as ::livemod::LiveModRepr<#field_type>>::repr(&#struct_name(#args), &self.#field_name) }
                } else {
                    quote! { ::livemod::LiveMod::repr_default(&self.#field_name) }
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
                            #name => &mut self.#field_name
                        },
                        quote! {
                            (#name.to_owned(), ::livemod::LiveMod::get_self(&self.#field_name))
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
                let field_type = field.ty;
                let repr = if let Some(Attr::Repr(struct_name, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    quote! { <_ as ::livemod::LiveModRepr<#field_type>>::repr(&#struct_name(#args), &self.#i) }
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
                let field_name = field.ident.unwrap();
                let name = attrs
                    .iter()
                    .filter_map(|attr| match attr {
                        Attr::Rename(name) => Some(name.clone()),
                        _ => None,
                    })
                    .next_back()
                    .unwrap_or({
                        let mut name = field_name.to_string();
                        name.as_mut_str()[..1].make_ascii_uppercase();
                        name = name.replace('_', " ");
                        name
                    });
                let field_type = field.ty;
                let repr = if let Some(Attr::Repr(struct_name, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    quote! { <#struct_name as ::livemod::LiveModRepr<#field_type>>::repr(&#struct_name(#args), &self) }
                } else {
                    quote! { ::livemod::LiveMod::repr_default(#field_name) }
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
                            #field_name
                        }
                    ),
                    (
                        quote! {
                            #name => #field_name
                        },
                        (
                            quote! {
                                (#name.to_owned(), ::livemod::LiveMod::get_self(#field_name))
                            },
                            quote! {
                                #field_name: Default::default()
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
                let field_type = field.ty;
                let repr = if let Some(Attr::Repr(struct_name, args)) =
                    attrs.iter().rfind(|attr| matches!(attr, Attr::Repr(_, _)))
                {
                    quote! { <#struct_name as ::livemod::LiveModRepr<#field_type>>::repr(&#struct_name(#args), &self) }
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

*/

enum Attr {
    Skip,
    Rename(String),
    Repr(TokenStream),
    Default(TokenStream),
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
            Ok(Attr::Repr(input.parse()?))
        } else {
            Err(syn::Error::new(
                attr_type.span(),
                "Unrecognised attribute tag",
            ))
        }
    }
}
