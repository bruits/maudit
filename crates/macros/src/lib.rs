use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{self, Parse, ParseStream, Parser as _, Result};
use syn::{parse_macro_input, Expr, ItemStruct};

struct Args {
    path: Expr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = input.parse()?;

        Ok(Args { path })
    }
}

#[proc_macro_attribute]
pub fn route(attrs: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let item_struct = syn::parse_macro_input!(item as ItemStruct);
    let attrs = syn::parse_macro_input!(attrs as Args);

    let struct_name = &item_struct.ident;
    let path = &attrs.path;

    let expanded = quote! {
        impl maudit::page::InternalPage for #struct_name {
            fn route_raw(&self) -> String {
                #path.to_string()
            }
        }

        impl maudit::page::FullPage for #struct_name {
            fn render_internal(&self, ctx: &mut maudit::page::RouteContext) -> maudit::page::RenderResult {
                self.render(ctx).into()
            }

            fn routes_internal(&self, ctx: &mut maudit::page::DynamicRouteContext) -> Vec<maudit::page::RouteParams> {
                self.routes(ctx).iter().map(Into::into).collect()
            }
        }

        #item_struct
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Params)]
pub fn derive_params(item: TokenStream) -> TokenStream {
    let item_struct = syn::parse_macro_input!(item as ItemStruct);
    let struct_name = &item_struct.ident;

    let fields = match &item_struct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| f.ident.as_ref().unwrap())
            .collect::<Vec<_>>(),
        _ => panic!("Only named fields are supported"),
    };

    // Add a from Hashmap conversion
    let expanded = quote! {
        impl From<RouteParams> for #struct_name {
            fn from(params: RouteParams) -> Self {
                #struct_name {
                    #(#fields: maudit::params::FromParam::from_param(params.0.get(stringify!(#fields)).unwrap()).unwrap(),)*
                }
            }
        }

        impl Into<RouteParams> for #struct_name {
            fn into(self) -> RouteParams {
                let mut map = maudit::FxHashMap::default();
                #(
                    map.insert(stringify!(#fields).to_string(), self.#fields.to_string());
                )*
                RouteParams(map)
            }
        }

        impl Into<RouteParams> for &#struct_name {
            fn into(self) -> RouteParams {
                let mut map = maudit::FxHashMap::default();
                #(
                    map.insert(stringify!(#fields).to_string(), self.#fields.to_string());
                )*
                RouteParams(map)
            }
        }

    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
// Helps implement a struct as a Markdown content entry.
//
// See complete documentation in `crates/framework/src/content.rs`.
pub fn markdown_entry(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_struct = syn::parse_macro_input!(item as ItemStruct);
    let _ = parse_macro_input!(args as parse::Nothing);

    let struct_name = &item_struct.ident;

    // Add __internal_headings field
    if let syn::Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! {
                    #[serde(skip)]
                    __internal_headings: Vec<maudit::content::MarkdownHeading>
                })
                .unwrap(),
        );
    }

    let expanded = quote! {
        #[derive(serde::Deserialize)]
        #item_struct

        impl maudit::content::MarkdownContent for #struct_name {
            fn get_headings(&self) -> &Vec<maudit::content::MarkdownHeading> {
                &self.__internal_headings
            }
        }

        impl maudit::content::InternalMarkdownContent for #struct_name {
            fn set_headings(&mut self, headings: Vec<maudit::content::MarkdownHeading>) {
                self.__internal_headings = headings;
            }
        }
    };

    TokenStream::from(expanded)
}
