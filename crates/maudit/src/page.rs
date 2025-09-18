//! Core traits and structs to define the pages of your website.
//!
//! Every page must implement the [`Page`] trait. Then, pages can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
use crate::assets::PageAssets;
use crate::build::finish_route;
use crate::content::PageContent;
use crate::route::{
    extract_params_from_raw_route, get_route_type_from_route_params, guess_if_route_is_endpoint,
};
use rustc_hash::FxHashMap;
use std::any::Any;

/// The result of a page render, can be either text or raw bytes.
///
/// Typically used through the [`Into<RenderResult>`](std::convert::Into) and [`From<RenderResult>`](std::convert::From) implementations for common types.
/// End users should rarely need to interact with this enum directly.
///
/// ## Example
/// ```rs
/// use maudit::page::prelude::*;
///
/// #[route("/")]
/// pub struct Index;
///
/// impl Page for Index {
///   fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///    "<h1>Hello, world!</h1>".into()
///   }
/// }
/// ```
pub enum RenderResult {
    Text(String),
    Raw(Vec<u8>),
}

impl From<maud::Markup> for RenderResult {
    fn from(val: maud::Markup) -> Self {
        RenderResult::Text(val.into_string())
    }
}

impl From<String> for RenderResult {
    fn from(val: String) -> Self {
        RenderResult::Text(val)
    }
}

impl From<&str> for RenderResult {
    fn from(val: &str) -> Self {
        RenderResult::Text(val.to_string())
    }
}

impl From<Vec<u8>> for RenderResult {
    fn from(val: Vec<u8>) -> Self {
        RenderResult::Raw(val)
    }
}

impl From<&[u8]> for RenderResult {
    fn from(val: &[u8]) -> Self {
        RenderResult::Raw(val.to_vec())
    }
}

pub type Routes<Params = RouteParams, Props = ()> = Vec<Route<Params, Props>>;

/// Represents a route with its parameters and associated props
#[derive(Debug, Clone)]
pub struct Route<Params = RouteParams, Props = ()>
where
    Params: Into<RouteParams>,
{
    pub params: Params,
    pub props: Props,
}

impl<Params, Props> Route<Params, Props>
where
    Params: Into<RouteParams>,
{
    pub fn new(params: Params, props: Props) -> Self {
        Self { params, props }
    }
}

impl<Params> Route<Params, ()>
where
    Params: Into<RouteParams>,
{
    pub fn from_params(params: Params) -> Self {
        Self { params, props: () }
    }
}

/// Pagination metadata for routes
#[derive(Debug, Clone)]
pub struct PaginationMeta {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_prev: bool,
    pub next_page: Option<usize>,
    pub prev_page: Option<usize>,
    pub start_index: usize,
    pub end_index: usize,
}

impl PaginationMeta {
    pub fn new(page: usize, per_page: usize, total_items: usize) -> Self {
        let total_pages = if total_items == 0 {
            1
        } else {
            total_items.div_ceil(per_page)
        };
        let start_index = page * per_page;
        let end_index = ((page + 1) * per_page).min(total_items);

        Self {
            page,
            per_page,
            total_items,
            total_pages,
            has_next: page < total_pages - 1,
            has_prev: page > 0,
            next_page: if page < total_pages - 1 {
                Some(page + 1)
            } else {
                None
            },
            prev_page: if page > 0 { Some(page - 1) } else { None },
            start_index,
            end_index,
        }
    }
}

/// Helper function to create paginated routes from a content source
pub fn paginate_content<T, Params>(
    entries: &[crate::content::ContentEntry<T>],
    per_page: usize,
    mut params_fn: impl FnMut(usize) -> Params,
) -> Routes<Params, PaginationMeta>
where
    Params: Into<RouteParams>,
{
    if entries.is_empty() {
        return vec![];
    }

    let total_items = entries.len();
    let total_pages = total_items.div_ceil(per_page);
    let mut routes = Vec::new();

    for page in 0..total_pages {
        let params = params_fn(page);
        let props = PaginationMeta::new(page, per_page, total_items);

        routes.push(Route::new(params, props));
    }

    routes
}

/// Helper to get paginated slice from content entries
pub fn get_page_slice<'a, T>(
    entries: &'a [crate::content::ContentEntry<T>],
    pagination: &'a PaginationMeta,
) -> &'a [crate::content::ContentEntry<T>] {
    &entries[pagination.start_index..pagination.end_index]
}

/// Allows to access various data and assets in a [`Page`] implementation.
///
/// ## Example
/// ```rs
/// use maudit::page::prelude::*;
/// use maud::html;
/// # use maudit::content::markdown_entry;
/// #
/// # #[markdown_entry]
/// # pub struct ArticleContent {
/// #    pub title: String,
/// #    pub description: String,
/// # }
///
/// #[route("/")]
/// pub struct Index;
///
/// impl Page for Index {
///   fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///     let logo = ctx.assets.add_image("logo.png");
///     let last_entries = &ctx.content.get_source::<ArticleContent>("articles").entries;
///     html! {
///       main {
///         (logo)
///         ul {
///           @for entry in last_entries {
///             li { (entry.data(ctx).title) }
///           }
///         }
///       }
///     }.into()
///   }
/// }
pub struct RouteContext<'a> {
    pub params: &'a dyn Any,
    pub props: &'a dyn Any,
    pub content: &'a PageContent<'a>,
    pub assets: &'a mut PageAssets,
    pub current_url: String,
}

impl<'a> RouteContext<'a> {
    pub fn from_static_route(
        content: &'a PageContent,
        assets: &'a mut PageAssets,
        current_url: String,
    ) -> Self {
        Self {
            params: &(),
            props: &(),
            content,
            assets,
            current_url,
        }
    }

    pub fn from_dynamic_route(
        dynamic_route: &'a RouteResult,
        content: &'a PageContent,
        assets: &'a mut PageAssets,
        current_url: String,
    ) -> Self {
        Self {
            params: dynamic_route.1.as_ref(),
            props: dynamic_route.2.as_ref(),
            content,
            assets,
            current_url,
        }
    }

    pub fn params<T: 'static + Clone>(&self) -> T {
        self.params
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Params type mismatch: got {}", std::any::type_name::<T>()))
            .clone()
    }

    pub fn props<T: 'static + Clone>(&self) -> T {
        self.props
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Props type mismatch: got {}", std::any::type_name::<T>()))
            .clone()
    }

    pub fn params_ref<T: 'static>(&self) -> &T {
        self.params
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Params type mismatch: got {}", std::any::type_name::<T>()))
    }

    pub fn props_ref<T: 'static>(&self) -> &T {
        self.props
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Props type mismatch: got {}", std::any::type_name::<T>()))
    }
}

/// Allows to access the content source in the [`Page::routes`] method.
///
/// ## Example
/// ```rs
/// use maudit::page::prelude::*;
/// # use maudit::content::markdown_entry;
/// #
/// # #[markdown_entry]
/// # pub struct ArticleContent {
/// #    pub title: String,
/// #    pub description: String,
/// # }
///
/// #[route("/articles/[article]")]
/// pub struct Article;
///
/// #[derive(Params)]
/// pub struct ArticleParams {
///     pub article: String,
/// }
///
/// impl Page<ArticleParams> for Article {
///    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///      let params = ctx.params::<ArticleParams>();
///      let articles = ctx.content.get_source::<ArticleContent>("articles");
///      let article = articles.get_entry(&params.article);
///      article.render().into()
///   }
///
///    fn routes(&self, ctx: &DynamicRouteContext) -> Vec<ArticleParams> {
///       let articles = ctx.content.get_source::<ArticleContent>("articles");
///
///       articles.into_params(|entry| ArticleParams {
///           article: entry.id.clone(),
///       })
///   }
/// }
/// ```
pub struct DynamicRouteContext<'a> {
    pub content: &'a PageContent<'a>,
}

/// Must be implemented for every page of your website.
///
/// The page struct implementing this trait can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
///
/// ## Example
/// ```rs
/// use maudit::page::prelude::*;
///
/// #[route("/")]
/// pub struct Index;
///
/// impl Page for Index {
///    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///       "<h1>Hello, world!</h1>".into()
///   }
/// }
/// ```
pub trait Page<Params = RouteParams, Props = (), T = RenderResult>
where
    Params: Into<RouteParams>,
    Props: 'static,
    T: Into<RenderResult>,
{
    fn routes(&self, _ctx: &DynamicRouteContext) -> Routes<Params, Props> {
        Vec::new()
    }
    fn render(&self, ctx: &mut RouteContext) -> T;
}

/// Raw representation of a route's parameters.
///
/// Can be accessed through [`RouteContext`]'s `raw_params`.
#[derive(Clone, Default, Debug)]
pub struct RouteParams(pub FxHashMap<String, String>);

impl RouteParams {
    pub fn from_vec<T>(params: Vec<T>) -> Vec<RouteParams>
    where
        T: Into<RouteParams>,
    {
        params.into_iter().map(|p| p.into()).collect()
    }
}

impl From<&RouteParams> for RouteParams {
    fn from(params: &RouteParams) -> Self {
        params.clone()
    }
}

impl<T> FromIterator<T> for RouteParams
where
    T: Into<RouteParams>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut map = FxHashMap::default();
        for item in iter {
            let item = item.into();
            map.extend(item.0);
        }
        RouteParams(map)
    }
}

#[derive(PartialEq, Eq, Debug)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::route`] implements it for the user behind the scenes.
pub enum RouteType {
    Static,
    Dynamic,
}

#[doc(hidden)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because the derive macro implements it for the user behind the scenes.
pub trait InternalPage {
    fn route_raw(&self) -> String;
    fn is_endpoint(&self) -> bool {
        guess_if_route_is_endpoint(&self.route_raw())
    }
    fn route_type(&self) -> RouteType {
        let params_def = extract_params_from_raw_route(&self.route_raw());

        get_route_type_from_route_params(&params_def)
    }

    fn url(&self, params: &RouteParams) -> String {
        let params_def = extract_params_from_raw_route(&self.route_raw());

        // Replace every param_def with the value from the params hashmap for said key
        // So, ex: "/articles/[article]" (params: Hashmap {article: "truc"}) -> "/articles/truc"
        let mut route = self.route_raw();

        for param_def in params_def {
            let value = params.0.get(&param_def.key);

            match value {
                Some(value) => {
                    route.replace_range(param_def.index..param_def.index + param_def.length, value);
                }
                None => {
                    panic!(
                        "Route {:?} is missing parameter {:?}",
                        self.route_raw(),
                        param_def.key
                    );
                }
            }
        }

        route
    }

    fn file_path(&self, params: &RouteParams) -> String {
        let params_def = extract_params_from_raw_route(&self.route_raw());
        let mut route = self.route_raw();

        for param_def in params_def {
            let value = params.0.get(&param_def.key);

            match value {
                Some(value) => {
                    route.replace_range(param_def.index..param_def.index + param_def.length, value);
                }
                None => {
                    panic!(
                        "Route {:?} is missing parameter {:?}",
                        self.route_raw(),
                        param_def.key
                    );
                }
            }
        }

        let cleaned_raw_route = route.trim_start_matches('/').to_string();

        match self.is_endpoint() {
            true => cleaned_raw_route,
            false => match cleaned_raw_route.is_empty() {
                true => "index.html".to_string(),
                false => format!("{}/index.html", cleaned_raw_route),
            },
        }
    }
}

/// Extension trait providing generic convenience methods on an instance of a page
pub trait PageExt<Params = RouteParams, Props = (), T = RenderResult>:
    Page<Params, Props, T> + InternalPage
where
    Params: Into<RouteParams>,
    Props: 'static,
    T: Into<RenderResult>,
{
    /// Get the URL for this page with the given parameters
    ///
    /// Note that this method merely generates the URL based on the route pattern and parameters, it does not verify if a corresponding route actually exists.
    fn url(&self, params: Params) -> String {
        InternalPage::url(self, &params.into())
    }
}

// Blanket implementation for all Page implementors that also implement InternalPage
impl<U, Params, Props, T> PageExt<Params, Props, T> for U
where
    U: Page<Params, Props, T> + InternalPage,
    Params: Into<RouteParams>,
    Props: 'static,
    T: Into<RenderResult>,
{
}

/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::route`] implements it for the user behind the scenes.
pub trait FullPage: InternalPage + Sync + Send {
    fn render_internal(&self, ctx: &mut RouteContext) -> RenderResult;
    fn routes_internal(&self, context: &DynamicRouteContext) -> RoutesResult;

    fn build(&self, ctx: &mut RouteContext) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let result = self.render_internal(ctx);
        let bytes = finish_route(result, ctx.assets, self.route_raw())?;

        Ok(bytes)
    }
}

pub type RouteResult = (RouteParams, RouteProps, RouteTypedParams);
pub type RoutesResult = Vec<RouteResult>;

pub type RouteProps = Box<dyn Any + Send + Sync>;
pub type RouteTypedParams = Box<dyn Any + Send + Sync>;

pub mod prelude {
    //! Re-exports of the most commonly used types and traits for defining pages.
    //!
    //! This module is meant to be glob imported in your page files.
    //!
    //! ## Example
    //! ```rs
    //! use maudit::page::prelude::*;
    //! ```
    pub use super::{
        DynamicRouteContext, FullPage, Page, PageExt, PaginationMeta, RenderResult, Route,
        RouteContext, RouteParams, Routes, get_page_slice, paginate_content,
    };
    pub use crate::assets::{Asset, Image, Style, StyleOptions};
    pub use crate::content::MarkdownContent;
    pub use maudit_macros::{Params, route};
}
