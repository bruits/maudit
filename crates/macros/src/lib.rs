use std::path::Path;

use proc_macro::TokenStream;
use quote::quote;
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
    let dynamic_values = extract_values(&attrs.path.value()).join(", ");

    let path = attrs.path.value();

    let file_path_for_route = url_to_file_path(&path, attrs.is_endpoint_file);

    let expanded = quote! {
        use std::path::{Path, PathBuf};
        use dire_coronet::page::{FullPage, InternalPage, Page, Params, RenderResult};

        impl InternalPage for #struct_name {
          fn route(&self) -> String {
            return #path.to_string();
          }

          fn file_path(&self) -> PathBuf {
            PathBuf::from(#file_path_for_route)
          }
        }

        impl FullPage for #struct_name {}

        impl Params for #struct_name {
            fn params (&self) -> Vec<String> {
                return vec![#dynamic_values.to_string()];
            }
        }

        #item_struct
    };

    TokenStream::from(expanded)
}

// Naive implementation to extract dynamic values from a path
fn extract_values(input: &str) -> Vec<String> {
    let input = input.trim_matches('"');
    let mut values = Vec::new();
    let mut start = false;
    let mut current_value = String::new();

    for c in input.chars() {
        match c {
            '[' => {
                start = true;
                current_value.clear();
            }
            ']' => {
                if start {
                    values.push(current_value.clone());
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

fn url_to_file_path(url: &str, is_file: bool) -> String {
    match is_file {
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
    }
}
