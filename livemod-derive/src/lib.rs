use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(LiveMod)]
pub fn livemod_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(st) => {
            match st.fields {
                syn::Fields::Named(fields) => {
                    let struct_name = ast.ident;
                    let (fields, matches) = fields.named.into_iter()
                        .filter_map(|field| {
                            if !field.attrs.into_iter().any(|attr| attr.path.is_ident("livemod")) {
                                //TODO: #[livemod(skip)]
                                //TODO: #[livemod(rename = "new_ident")]
                                //TODO: #[livemod(preserve_case)]
                                let ident = field.ident.unwrap();
                                let name = {
                                    let mut name = ident.to_string();
                                    name.as_mut_str()[..1].make_ascii_uppercase();
                                    name
                                };
                                Some((
                                    quote! {
                                        ::livemod::StructData {
                                            name: String::from(#name),
                                            data_type: ::livemod::LiveMod::data_type(&self.#ident)
                                        }
                                    },
                                    quote! {
                                        #name => &mut self.#ident
                                    }
                                ))
                            } else {
                                None
                            }
                        })
                        .unzip::<_, _, Vec<_>, Vec<_>>();
                    let gen = quote! {
                        impl ::livemod::LiveMod for #struct_name {
                            fn data_type(&self) -> ::livemod::StructDataType {
                                ::livemod::StructDataType::Struct {
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

                            fn set_self(&mut self, value: ::livemod::StructDataValue) {
                                panic!("Unexpected set operation!")
                            }
                        }
                    };
                    gen.into()
                },
                syn::Fields::Unnamed(fields) => todo!(),
                syn::Fields::Unit => todo!(),
            }
        },
        syn::Data::Enum(en) => todo!(),
        syn::Data::Union(_) => todo!(),
    }
}