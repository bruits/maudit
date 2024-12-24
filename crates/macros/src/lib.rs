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
            impl maudit::page::DynamicPage for #struct_name {
                fn routes(&self) -> Vec<maudit::page::RouteParams> {
                    Vec::new()
                }
            }
        },
    };

    let path = attrs.path.value();

    let list_params = params
        .iter()
        .map(|v| {
            let key = format_ident!("{}", v.key);
            quote! { let #key = params.0.get(stringify!(#key)).unwrap().to_string() }
        })
        .collect::<Vec<_>>();

    let struct_def_params = params
        .iter()
        .map(|v| {
            let key = format_ident!("{}", v.key);
            quote! { #key: String }
        })
        .collect::<Vec<_>>();

    let path_for_route = make_params_dynamic(&path, &params, 0);
    let file_path_for_route = url_to_file_path(&path, attrs.is_endpoint_file, &params);

    let expanded = quote! {
        struct RawParams {
            #(#struct_def_params,)*
        }

        impl RawParams {
            fn get_field_names() -> Vec<&'static str> {
                vec![#(stringify!(#struct_def_params)),*]
            }
        }

        impl maudit::page::InternalPage for #struct_name {
            fn route_raw(&self) -> String {
                #path.to_string()
            }

            fn route(&self, params: &maudit::page::RouteParams) -> String {
                #(#list_params;)*
                return format!(#path_for_route);
            }

            fn file_path(&self, params: &maudit::page::RouteParams) -> std::path::PathBuf {
                #(#list_params;)*
                std::path::PathBuf::from(format!(#file_path_for_route))
            }

            fn url<P: Into<maudit::page::RouteParams>>(params: P) -> String {
                let params = params.into();
                #(#list_params;)*
                format!(#path_for_route)
            }
        }


        #dynamic_page_impl

        impl maudit::page::FullPage for #struct_name {}

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

fn url_to_file_path(url: &str, is_file: bool, params: &[Parameter]) -> String {
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

    make_params_dynamic(&file_path, params, 1)
}

fn make_params_dynamic(file_path: &str, params: &[Parameter], offset: usize) -> String {
    let mut file_path = file_path.to_string();
    for param in params.iter().rev() {
        file_path.replace_range(
            param.index - offset..param.index + param.length - offset,
            &format!("{{{}}}", param.key),
        );
    }

    file_path
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
