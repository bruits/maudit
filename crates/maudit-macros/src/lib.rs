use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{self, Parse, ParseStream, Parser as _, Result};
use syn::{Expr, Ident, ItemStruct, Token, parse_macro_input, punctuated::Punctuated};

enum LocaleKind {
    FullPath(Expr),
    Prefix(Expr),
}

struct LocaleVariant {
    locale: Ident,
    kind: LocaleKind,
}

impl Parse for LocaleVariant {
    fn parse(input: ParseStream) -> Result<Self> {
        let locale = input.parse::<Ident>()?;

        // Check if it's `locale = "path"`, `locale(path = "path")`, or `locale(prefix = "path")`
        let lookahead = input.lookahead1();

        let kind = if lookahead.peek(Token![=]) {
            // Shorthand full path: `en = "/en/about"`
            input.parse::<Token![=]>()?;
            let path = input.parse::<Expr>()?;
            LocaleKind::FullPath(path)
        } else if lookahead.peek(syn::token::Paren) {
            // Either `en(path = "...")` or `en(prefix = "...")`
            let content;
            syn::parenthesized!(content in input);

            let key_ident: Ident = content.parse()?;
            content.parse::<Token![=]>()?;
            let value = content.parse::<Expr>()?;

            if key_ident == "path" {
                LocaleKind::FullPath(value)
            } else if key_ident == "prefix" {
                LocaleKind::Prefix(value)
            } else {
                return Err(content.error("expected 'path' or 'prefix'"));
            }
        } else {
            return Err(lookahead.error());
        };

        Ok(LocaleVariant { locale, kind })
    }
}

struct RouteArgs {
    path: Option<Expr>,
    locales: Vec<LocaleVariant>,
}

impl Parse for RouteArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut path = None;
        let mut locales = Vec::new();

        if input.is_empty() {
            return Ok(RouteArgs { path, locales });
        }

        // First argument: either a path expression or a named argument like locales(...)
        if input.peek(Ident) && input.peek2(syn::token::Paren) {
            // If the first argument is a named one, that means there's no base path and this route should only have variants
            let ident: Ident = input.parse()?;
            let ident_str = ident.to_string();

            if ident_str == "locales" {
                let content;
                syn::parenthesized!(content in input);
                let variants = Punctuated::<LocaleVariant, Token![,]>::parse_terminated(&content)?;
                locales = variants.into_iter().collect();
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    format!("unknown argument '{}', expected 'locales'", ident_str),
                ));
            }
        } else {
            // First argument is a path expression, e.g., "/about" so proceed as normal
            path = Some(input.parse::<Expr>()?);
        }

        // Parse remaining named arguments (right now just locales(...))
        while !input.is_empty() {
            input.parse::<Token![,]>()?;

            if input.is_empty() {
                break;
            }

            // All subsequent arguments must be named (e.g., locales(...), the path must be first)
            if input.peek(Ident) && input.peek2(syn::token::Paren) {
                let ident: Ident = input.parse()?;
                let ident_str = ident.to_string();

                if ident_str == "locales" {
                    if !locales.is_empty() {
                        return Err(syn::Error::new_spanned(
                            ident,
                            "locales specified multiple times",
                        ));
                    }
                    let content;
                    syn::parenthesized!(content in input);
                    let variants =
                        Punctuated::<LocaleVariant, Token![,]>::parse_terminated(&content)?;
                    locales = variants.into_iter().collect();
                } else {
                    return Err(syn::Error::new_spanned(
                        ident,
                        format!("unknown argument '{}'", ident_str),
                    ));
                }
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "expected named argument (e.g., locales(...)), path must be first argument",
                ));
            }
        }

        // Check for duplicate locales
        Self::check_duplicate_locales(&locales)?;

        Ok(RouteArgs { path, locales })
    }
}

impl RouteArgs {
    fn check_duplicate_locales(locales: &[LocaleVariant]) -> Result<()> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();

        for variant in locales {
            let locale_name = variant.locale.to_string();
            if !seen.insert(locale_name.clone()) {
                return Err(syn::Error::new_spanned(
                    &variant.locale,
                    format!("duplicate locale '{}' specified", locale_name),
                ));
            }
        }

        Ok(())
    }
}

#[proc_macro_attribute]
pub fn route(attrs: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let item_struct = syn::parse_macro_input!(item as ItemStruct);
    let args = syn::parse_macro_input!(attrs as RouteArgs);

    let struct_name = &item_struct.ident;

    // Generate variants method based on locales
    let variant_method = if !args.locales.is_empty() {
        let variant_tuples = args.locales.iter().map(|variant| {
            let locale_name = variant.locale.to_string();

            match &variant.kind {
                LocaleKind::FullPath(path) => {
                    quote! {
                        (#locale_name.to_string(), #path.to_string())
                    }
                }
                LocaleKind::Prefix(prefix) => {
                    if args.path.is_none() {
                        // Emit compile error if prefix is used without base path
                        quote! {
                            compile_error!("Cannot use locale prefix without a base route path")
                        }
                    } else {
                        let base_path = args.path.as_ref().unwrap();
                        quote! {
                            (#locale_name.to_string(), format!("{}{}", #prefix, #base_path))
                        }
                    }
                }
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
    let route_raw_impl = if let Some(path) = &args.path {
        quote! {
            fn route_raw(&self) -> Option<String> {
                Some(#path.to_string())
            }
        }
    } else {
        quote! {
            fn route_raw(&self) -> Option<String> {
                None
            }
        }
    };

    let expanded = quote! {
        impl maudit::route::InternalRoute for #struct_name {
            #route_raw_impl

            #variant_method
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
