use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parenthesized, parse::Parse, punctuated::Punctuated, DeriveInput, LitStr, Token};

#[proc_macro_derive(LiveMod, attributes(livemod))]
pub fn livemod_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => match st.fields {
            syn::Fields::Named(fields) => {
                let struct_name = ast.ident;
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
                                quote! { ::livemod::LiveMod::data_type(&self.#ident) }
                            };
                            Some((
                                quote! {
                                    ::livemod::TrackedData {
                                        name: String::from(#name),
                                        data_type: #repr,
                                        modifies_structure: false,
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
                let gen = quote! {
                    #[automatically_derived]
                    impl ::livemod::LiveMod for #struct_name {
                        fn data_type(&self) -> ::livemod::TrackedDataRepr {
                            ::livemod::TrackedDataRepr::Struct {
                                name: String::from(stringify!(#struct_name)),
                                fields: vec![
                                    #(#fields),*
                                ],
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

                        fn get_self(&self) -> ::livemod::TrackedDataValue {
                            ::livemod::TrackedDataValue::Struct(vec![
                                #(#gets),*
                            ])
                        }
                    }
                };
                gen.into()
            }
            syn::Fields::Unnamed(_fields) => todo!(),
            syn::Fields::Unit => todo!(),
        },
        syn::Data::Enum(_en) => todo!(),
        syn::Data::Union(_) => todo!(),
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
                Err(input.error("Expected end of attribute content"))?;
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
