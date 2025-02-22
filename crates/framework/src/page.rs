//! Core traits and structs to define the pages of your website.
//!
//! Every page must implement the [`Page`] trait. Then, pages can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
use crate::assets::PageAssets;
use crate::content::Content;
use crate::route::{extract_params_from_raw_route, get_route_url, guess_if_route_is_endpoint};
use rustc_hash::FxHashMap;

/// Represents the result of a page render, can be either text or raw bytes.
///
/// Typically used through the [`Into<RenderResult>`](std::convert::Into) and [`From<RenderResult>`](std::convert::From) implementations for common types.
/// End users should rarely need to interact with this enum directly.
///
/// ## Example
/// ```rust
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

/// Allows to access various data and assets in a [`Page`] implementation.
///
/// ## Example
/// ```rust
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
///             li { (entry.data.title) }
///           }
///         }
///       }
///     }.into()
///   }
/// }
pub struct RouteContext<'a> {
    pub raw_params: &'a RouteParams,
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
}

/// Allows to access the content source in the [`Page::routes`] method.
///
/// ## Example
/// ```rust
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
/// ```rust
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
pub trait Page<P = RouteParams, T = RenderResult>
where
    P: Into<RouteParams>,
    T: Into<RenderResult>,
{
    fn routes(&self, _ctx: &mut DynamicRouteContext) -> Vec<P> {
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
    fn routes_internal(&self, context: &mut DynamicRouteContext) -> Vec<RouteParams>;
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
    //! ```rust
    //! use maudit::page::prelude::*;
    //! ```
    pub use super::{
        get_page_url, DynamicRouteContext, Page, RenderResult, RouteContext, RouteParams,
    };
    #[doc(hidden)]
    pub use super::{FullPage, InternalPage};
    pub use crate::assets::Asset;
    pub use crate::content::MarkdownContent;
    pub use maudit_macros::{route, Params};
}
