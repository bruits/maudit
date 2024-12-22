use std::path::Path;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::{ItemStruct, LitStr};

struct Args {
    path: LitStr,
    is_endpoint_file: bool,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = input.parse::<LitStr>()?;

        // If the path ends with a file extension, it is a file, handle any file extensions

        let binding = path.value();
        let real_path = Path::new(&binding);

        Ok(Args {
            path,
            is_endpoint_file: real_path.extension().is_some(),
        })
    }
}

#[proc_macro_attribute]
pub fn route(attrs: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let item_struct = syn::parse_macro_input!(item as ItemStruct);
    let attrs = syn::parse_macro_input!(attrs as Args);

    let struct_name = &item_struct.ident;

    let params = extract_values(&attrs.path.value());

    let dynamic_page_impl = match params.is_empty() {
        false => quote! {},
        true => quote! {
            impl DynamicPage for #struct_name {
                fn routes(&self) -> std::collections::HashMap<String, String> {
                    let mut routes = std::collections::HashMap::new();
                    routes
                }
            }
        },
    };

    let path = attrs.path.value();

    let list_params = params
        .iter()
        .map(|v| {
            let key = format_ident!("{}", v.key);
            quote! { let #key = params.get(stringify!(#key)).unwrap().to_string() }
        })
        .collect::<Vec<_>>();

    let path_for_route = make_params_dynamic(&path, &params, 0);
    let file_path_for_route = url_to_file_path(&path, attrs.is_endpoint_file, &params);

    let expanded = quote! {
        use std::path::{Path, PathBuf};
        use maudit::page::{FullPage, InternalPage, Page, RenderResult, DynamicPage};

        impl InternalPage for #struct_name {
                    fn route_raw(&self) -> String {
                        #path.to_string()
                    }

          fn route(&self, params: std::collections::HashMap<String, String>) -> String {
                        #(#list_params;)*
            return format!(#path_for_route);
          }

          fn file_path(&self, params: std::collections::HashMap<String, String>) -> PathBuf {
                        // List params in the shape of let id = ctx.params.get("id").unwrap().to_string();
                        #(#list_params;)*
            PathBuf::from(format!(#file_path_for_route))
          }
        }


            #dynamic_page_impl

        impl FullPage for #struct_name {}

        #item_struct
    };

    TokenStream::from(expanded)
}

struct Parameter {
    key: String,
    index: usize,
    length: usize,
}

// Naive implementation to extract dynamic values from a path
fn extract_values(input: &str) -> Vec<Parameter> {
    let input = input.trim_matches('"');
    let mut values = Vec::new();
    let mut start = false;
    let mut current_value = String::new();
    let mut start_index = 0;

    for (i, c) in input.chars().enumerate() {
        match c {
            '[' => {
                start = true;
                current_value.clear();
                start_index = i;
            }
            ']' => {
                if start {
                    values.push(Parameter {
                        key: current_value.clone(),
                        index: start_index,
                        length: i - start_index + 1,
                    });
                    start = false;
                }
            }
            _ => {
                if start {
                    current_value.push(c);
                }
            }
        }
    }

    values
}

fn url_to_file_path(url: &str, is_file: bool, params: &Vec<Parameter>) -> String {
    let file_path = match is_file {
        false => {
            // Remove the leading '/' from the URL if it exists
            let path_str = url.trim_start_matches('/');

            // If the URL is empty (i.e., root), return "index.html"
            if path_str.is_empty() {
                return "index.html".to_string();
            }

            format!("{}/index.html", path_str)
        }
        true => {
            // Remove the leading '/' from the URL if it exists
            let path_str = url.trim_start_matches('/');

            // If the URL is empty (i.e., root), return "index.html"
            if path_str.is_empty() {
                panic!("Invalid file path");
            }

            path_str.to_string()
        }
    };

    make_params_dynamic(&file_path, &params, 1)
}

fn make_params_dynamic(file_path: &str, params: &Vec<Parameter>, offset: usize) -> String {
    let mut file_path = file_path.to_string();
    for param in params.iter().rev() {
        file_path.replace_range(
            param.index - offset..param.index + param.length - offset,
            &format!("{{{}}}", param.key),
        );
    }

    file_path
}
