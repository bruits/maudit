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
        impl maudit::route::InternalRoute for #struct_name {
            fn route_raw(&self) -> String {
                #path.to_string()
            }
        }

        impl maudit::route::FullRoute for #struct_name {
            fn render_internal(&self, ctx: &mut maudit::route::PageContext) -> Result<maudit::route::RenderResult, Box<dyn std::error::Error>> {
                let result: maudit::route::RenderResult = self.render(ctx).into();
                result.into()
            }

            fn pages_internal(&self, ctx: &mut maudit::route::DynamicRouteContext) -> Vec<(maudit::route::PageParams, Box<dyn std::any::Any + Send + Sync>, Box<dyn std::any::Any + Send + Sync>)> {
                self.pages(ctx)
                    .into_iter()
                    .map(|route| {
                        let raw_params: maudit::route::PageParams = (&route.params).into();
                        let typed_params: Box<dyn std::any::Any + Send + Sync> = Box::new(route.params);
                        let props: Box<dyn std::any::Any + Send + Sync> = Box::new(route.props);
                        (raw_params, typed_params, props)
                    })
                    .collect()
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

    let field_conversions = match &item_struct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();

                // Check if the field type is Option<T>
                if is_option_type(&field.ty) {
                    quote! {
                        map.insert(
                            #field_name_str.to_string(),
                            self.#field_name.as_ref().map_or("__MAUDIT_NONE__".to_string(), |v| v.to_string())
                        );
                    }
                } else {
                    quote! {
                        map.insert(#field_name_str.to_string(), self.#field_name.to_string());
                    }
                }
            })
            .collect::<Vec<_>>(),
        _ => panic!("Only named fields are supported"),
    };

    let expanded = quote! {
        impl Into<PageParams> for #struct_name {
            fn into(self) -> PageParams {
                (&self).into()
            }
        }

        impl Into<PageParams> for &#struct_name {
            fn into(self) -> PageParams {
                let mut map = maudit::FxHashMap::default();
                #(#field_conversions)*
                PageParams(map)
            }
        }
    };

    TokenStream::from(expanded)
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

#[proc_macro_attribute]
// Helps implement a struct as a Markdown content entry.
//
// See complete documentation in `crates/maudit/src/content.rs`.
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
