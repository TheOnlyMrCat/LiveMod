use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use syn::{
    parenthesized, parse::Parse, punctuated::Punctuated, DataEnum, DeriveInput, Field, FieldsNamed,
    FieldsUnnamed, LitStr, Token,
};

#[proc_macro_derive(LiveMod, attributes(livemod))]
pub fn livemod_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => {
            let struct_name = ast.ident;
            let (
                FieldsDerive {
                    idents,
                    default_values,
                    representations,
                    get_named_values,
                    get_selves,
                },
                named,
            ) = match st.fields {
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
                    fn repr_default(&self, target: ::livemod::ActionTarget) -> ::livemod::TrackedDataRepr {
                        let #self_pattern = self;
                        if let Some((field, field_target)) = target.strip_one_field() {
                            match field {
                                #(#get_named_values as &dyn ::livemod::LiveMod,)*
                                _ => panic!("Unexpected value name!"),
                            }.repr_default(field_target)
                        } else {
                            ::livemod::TrackedDataRepr::Struct {
                                name: String::from(stringify!(#struct_name)),
                                fields: vec![
                                    #(#representations),*
                                ],
                                triggers: vec![]
                            }
                        }
                    }

                    fn trigger(&mut self, target: ::livemod::ActionTarget, trigger: ::livemod::Trigger) -> bool {
                        panic!("Unexpected trigger operation!")
                    }

                    fn get_self(&self, target: ::livemod::ActionTarget) -> ::livemod::TrackedDataValue {
                        let #self_pattern = self;
                        if let Some((field, field_target)) = target.strip_one_field() {
                            match field {
                                #(#get_named_values as &dyn ::livemod::LiveMod,)*
                                _ => panic!("Unexpected value name!"),
                            }.get_self(field_target)
                        } else {
                            ::livemod::TrackedDataValue::Struct(vec![
                                #(#get_selves),*
                            ])
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Enum(en) => {
            let enum_name = ast.ident;

            let mut variant_names = vec![];
            let mut variant_fields = vec![];
            let mut variant_get_named_values = vec![];
            let mut variant_defaults = vec![];
            let mut variant_get_selves = vec![];

            for variant in en.variants {
                let variant_name = variant.ident;
                let variant_string = variant_name.to_string();
                variant_names.push(variant_name.to_string());
                match variant.fields {
                    syn::Fields::Named(fields) => {
                        let FieldsDerive {
                            idents,
                            default_values,
                            representations,
                            get_named_values,
                            get_selves,
                        } = derive_fields_named(fields);
                        let self_pattern = quote! {
                            Self::#variant_name { #(#idents),* }
                        };

                        variant_fields
                            .push(quote! { #self_pattern => vec![#(#representations),*] });
                        variant_get_named_values.push(quote! { #self_pattern => match name { #(#get_named_values ,)* _ => panic!("Unexpected value name!") } });
                        variant_defaults.push(quote! { #variant_string => Self::#variant_name { #(#idents: #default_values),* } });
                        variant_get_selves.push(quote! { #self_pattern => ::livemod::TrackedDataValue::Enum { variant: #variant_string.to_owned(), fields: vec![#(#get_selves),*] } })
                    }
                    syn::Fields::Unnamed(fields) => {
                        let FieldsDerive {
                            idents,
                            default_values,
                            representations,
                            get_named_values,
                            get_selves,
                        } = derive_fields_unnamed(fields);
                        let self_pattern = quote! {
                            Self::#variant_name ( #(#idents),* )
                        };

                        variant_fields
                            .push(quote! { #self_pattern => vec![#(#representations),*] });
                        variant_get_named_values.push(quote! { #self_pattern => match name { #(#get_named_values ,)* _ => panic!("Unexpected value name!") } });
                        variant_defaults.push(quote! { #variant_string => Self::#variant_name ( #(#default_values),* ) });
                        variant_get_selves.push(quote! { #self_pattern => ::livemod::TrackedDataValue::Enum { variant: #variant_string.to_owned(), fields: vec![#(#get_selves),*] } })
                    }
                    syn::Fields::Unit => {
                        variant_fields.push(quote! { Self::#variant_name => vec![] });
                        variant_get_named_values.push(
                            quote! { Self::#variant_name => panic!("Unexpected value name!") },
                        );
                        variant_defaults.push(quote! { #variant_string => Self::#variant_name });
                        variant_get_selves.push(quote! { Self::#variant_name => ::livemod::TrackedDataValue::Enum { variant: #variant_string.to_owned(), fields: vec![] } })
                    }
                }
            }

            let gen = quote! {
                #[automatically_derived]
                impl ::livemod::LiveMod for #enum_name {
                    fn repr_default(&self) -> ::livemod::TrackedDataRepr {
                        ::livemod::TrackedDataRepr::Enum {
                            name: String::from(stringify!(#enum_name)),
                            variants: vec![
                                #(#variant_names.to_owned()),*
                            ],
                            fields: match self {
                                #(#variant_fields ,)*
                            },
                            triggers: vec![]
                        }
                    }

                    fn get_named_value(&mut self, name: &str) -> &mut dyn ::livemod::LiveMod {
                        match self {
                            #(#variant_get_named_values ,)*
                        }
                    }

                    fn trigger(&mut self, trigger: ::livemod::Trigger) -> bool {
                        let variant_name = trigger.try_into_set().unwrap().try_into_enum_variant().unwrap();
                        *self = match variant_name.as_str() {
                            #(#variant_defaults ,)*
                            name => panic!("Unknown variant name: {}", name)
                        };
                        true
                    }

                    fn get_self(&self) -> ::livemod::TrackedDataValue {
                        match self {
                            #(#variant_get_selves ,)*
                        }
                    }
                }
            };
            gen.into()
        }
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

fn derive_field(ident: Ident, default_name: String, field: Field) -> FieldDerive {
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

    let default_value = if let Some(default) = attrs.iter().find_map(|attr| match attr {
        Attr::Default(ts) => Some(ts),
        _ => None,
    }) {
        default.clone()
    } else {
        quote! { ::std::default::Default::default() }
    };

    let name = if let Some(name) = attrs.iter().find_map(|attr| match attr {
        Attr::Rename(name) => Some(name),
        _ => None,
    }) {
        name.clone()
    } else {
        default_name
    };

    let (representation, get_named_value, get_self) =
        if attrs.iter().any(|attr| matches!(attr, Attr::Skip)) {
            (None, None, None)
        } else {
            let default_repr = quote! { ::livemod::DefaultRepr };
            let repr_struct = attrs
                .iter()
                .find_map(|attr| match attr {
                    Attr::Repr(ts) => Some(ts),
                    _ => None,
                })
                .unwrap_or(&default_repr);
            let representation = quote! {
                ::livemod::TrackedData {
                    name: #name.to_owned(),
                    data_type: ::livemod::LiveModRepr::repr(&#repr_struct, #ident) ,
                    triggers: vec![]
                }
            };

            let get_named_value = quote! { #name => #ident };
            let get_self = quote! { (#name.to_owned(), ::livemod::LiveMod::get_self(#ident, ::livemod::ActionTarget::This)) };
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
        } else if attr_type == "default" {
            input.parse::<Token![=]>()?;
            Ok(Attr::Default(input.parse()?))
        } else {
            Err(syn::Error::new(
                attr_type.span(),
                "Unrecognised attribute tag",
            ))
        }
    }
}
