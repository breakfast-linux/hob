use proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident};

#[proc_macro_derive(ObjectTraversal, attributes(skip))]
pub fn derive_object_traversal(token_stream: TokenStream) -> TokenStream {
    let ast = syn::parse::<DeriveInput>(token_stream).unwrap();

    let name = &ast.ident;
    let str_n = name;

    let body = match &ast.data {
        Data::Struct(str) => {
            let mut traverse = vec![];
            for field in &str.fields {
                if field
                    .attrs
                    .iter()
                    .any(|x| x.path.segments[0].ident.to_string() == "skip")
                {
                    continue;
                }

                let name = field.ident.as_ref().unwrap();
                traverse.push(quote::quote! {
                    self.#name.traverse(walker);
                });
            }

            traverse
        }

        Data::Enum(en) => {
            let mut trav = vec![];
            for var in &en.variants {
                let name = &var.ident;

                let traverse = match &var.fields {
                    Fields::Named(named) => {
                        let fields: Vec<Ident> = named
                            .named
                            .iter()
                            .map(|x| x.ident.as_ref().unwrap().clone())
                            .collect();

                        quote::quote! {
                            #str_n::#name { #(#fields),* } => {
                                #(#fields.traverse(walker);)*
                            }
                        }
                    }

                    Fields::Unnamed(unnamed) => {
                        let fields: Vec<_> = unnamed
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(idx, _)| {
                                Ident::new(&format!("f{}", idx), proc_macro2::Span::call_site())
                            })
                            .collect();

                        quote::quote! {
                            #str_n::#name(#(#fields),*) => {
                                #(#fields.traverse(walker);)*
                            }
                        }
                    }

                    Fields::Unit => {
                        quote::quote! {
                            #str_n::#name => {}
                        }
                    }
                };

                trav.push(traverse);
            }

            vec![quote! {
                match self {
                    #(#trav),*
                }
            }]
        }

        _ => {
            vec![]
        }
    };

    let q = quote! {
        impl ::hob_utils::ObjectTraversal for #name {
            fn traverse<W: ::hob_utils::ObjectWalker>(&mut self, walker: &mut W) {
                #(#body)*;
            }
        }
    };

    q.into()
}
