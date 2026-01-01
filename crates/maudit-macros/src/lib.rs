use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{self, Parse, ParseStream, Parser as _, Result};
use syn::{Expr, Ident, ItemStruct, Token, parse_macro_input, punctuated::Punctuated};

struct Args {
    path: Option<Expr>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            Ok(Args { path: None })
        } else {
            let path = input.parse()?;
            Ok(Args { path: Some(path) })
        }
    }
}

struct LocaleVariant {
    locale: Ident,
    path: Expr,
}

impl Parse for LocaleVariant {
    fn parse(input: ParseStream) -> Result<Self> {
        let locale = input.parse::<Ident>()?;

        let content;
        syn::parenthesized!(content in input);

        content.parse::<Ident>()?; // "path"
        content.parse::<Token![=]>()?;
        let path = content.parse::<Expr>()?;

        Ok(LocaleVariant { locale, path })
    }
}

struct LocalesArgs {
    variants: Punctuated<LocaleVariant, Token![,]>,
}

impl Parse for LocalesArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let variants = Punctuated::parse_terminated(input)?;
        Ok(LocalesArgs { variants })
    }
}

#[proc_macro_attribute]
pub fn locales(attrs: TokenStream, item: TokenStream) -> TokenStream {
    // Parse and validate the locales
    let locales_args = syn::parse_macro_input!(attrs as LocalesArgs);
    let item_struct = syn::parse_macro_input!(item as ItemStruct);

    // Serialize the locale data into a doc comment that route macro can parse
    let mut locale_data = String::from("maudit_locales:");
    for variant in &locales_args.variants {
        let locale_name = variant.locale.to_string();
        let locale_path = match &variant.path {
            Expr::Lit(lit) => {
                if let syn::Lit::Str(s) = &lit.lit {
                    s.value()
                } else {
                    panic!("locale path must be a string literal");
                }
            }
            _ => panic!("locale path must be a string literal"),
        };
        locale_data.push_str(&format!("{}={},", locale_name, locale_path));
    }

    // Add the doc comment to the struct's attributes
    let mut modified_struct = item_struct.clone();
    modified_struct.attrs.push(syn::parse_quote! {
        #[doc = #locale_data]
    });

    let expanded = quote! {
        #modified_struct
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn route(attrs: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let item_struct = syn::parse_macro_input!(item as ItemStruct);
    let attrs = syn::parse_macro_input!(attrs as Args);

    let struct_name = &item_struct.ident;

    // Look for locale data in doc comments (set by locales macro)
    let locale_data = item_struct.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("doc")
            && let syn::Meta::NameValue(meta) = &attr.meta
            && let Expr::Lit(lit) = &meta.value
            && let syn::Lit::Str(s) = &lit.lit
        {
            let content = s.value();
            if content.starts_with("maudit_locales:") {
                return Some(content);
            }
        }
        None
    });

    let variant_methods = if let Some(locale_data) = locale_data {
        // Parse the locale data from the doc comment
        let data = locale_data.strip_prefix("maudit_locales:").unwrap();
        let mut variants = Vec::new();

        for pair in data.split(',') {
            if pair.is_empty() {
                continue;
            }
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() == 2 {
                let id = parts[0].to_string();
                let path = parts[1].to_string();
                variants.push((id, path));
            }
        }

        let variant_tuples = variants.iter().map(|(id, path)| {
            quote! {
                (#id.to_string(), #path.to_string())
            }
        });

        quote! {
            fn variants(&self) -> Vec<(String, String)> {
                vec![#(#variant_tuples),*]
            }
        }
    } else {
        quote! {
            fn variants(&self) -> Vec<(String, String)> {
                vec![]
            }
        }
    };

    // Generate route_raw implementation based on whether path is provided
    let route_raw_impl = if let Some(path) = &attrs.path {
        quote! {
            fn route_raw(&self) -> String {
                #path.to_string()
            }
        }
    } else {
        quote! {
            fn route_raw(&self) -> String {
                String::new()
            }
        }
    };

    let expanded = quote! {
        impl maudit::route::InternalRoute for #struct_name {
            #route_raw_impl

            #variant_methods
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
                            self.#field_name.as_ref().map(|v| v.to_string())
                        );
                    }
                } else {
                    quote! {
                        map.insert(#field_name_str.to_string(), Some(self.#field_name.to_string()));
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
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
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
