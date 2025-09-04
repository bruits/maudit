//! Core traits and structs to define the pages of your website.
//!
//! Every page must implement the [`Page`] trait. Then, pages can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
use crate::assets::PageAssets;
use crate::content::Content;
use crate::route::{extract_params_from_raw_route, get_route_url, guess_if_route_is_endpoint};
use rustc_hash::FxHashMap;
use std::any::Any;

/// Represents the result of a page render, can be either text or raw bytes.
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
) -> Vec<Route<Params, PaginationMeta>>
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
    pub raw_params: &'a RouteParams,
    pub props: &'a dyn Any,
    pub content: &'a Content<'a>,
    pub assets: &'a mut PageAssets,
    pub current_url: String,
}

impl RouteContext<'_> {
    pub fn params<T>(&self) -> T
    where
        T: From<RouteParams>,
    {
        T::from(self.raw_params.clone())
    }

    pub fn typed_props<T: 'static>(&self) -> &T {
        self.props.downcast_ref::<T>().expect("Props type mismatch")
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
///    fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<ArticleParams> {
///       let articles = ctx.content.get_source::<ArticleContent>("articles");
///
///       articles.into_params(|entry| ArticleParams {
///           article: entry.id.clone(),
///       })
///   }
/// }
/// ```
pub struct DynamicRouteContext<'a> {
    pub content: &'a mut Content<'a>,
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
    fn routes(&self, _ctx: &mut DynamicRouteContext) -> Vec<Route<Params, Props>> {
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

#[doc(hidden)]
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
}

#[doc(hidden)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::route`] implements it for the user behind the scenes.
pub trait FullPage: InternalPage + Sync {
    fn render_internal(&self, ctx: &mut RouteContext) -> RenderResult;
    fn routes_internal(
        &self,
        context: &mut DynamicRouteContext,
    ) -> Vec<(RouteParams, Box<dyn Any + Send + Sync>)>;
}

pub fn get_page_url<T: Into<RouteParams>>(route: &impl FullPage, params: T) -> String {
    let params_defs = extract_params_from_raw_route(&route.route_raw());
    format!(
        "/{}",
        get_route_url(&route.route_raw(), &params_defs, &params.into())
    )
}

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
        DynamicRouteContext, Page, PaginationMeta, RenderResult, Route, RouteContext, RouteParams,
        get_page_slice, get_page_url, paginate_content,
    };
    // TODO: Remove this internal re-export when possible
    #[doc(hidden)]
    pub use super::{FullPage, InternalPage};
    pub use crate::assets::{Asset, Image, Style, StyleOptions};
    pub use crate::content::MarkdownContent;
    pub use maudit_macros::{Params, route};
}
