//! Core traits and structs to define the pages of your website.
//!
//! Every page must implement the [`Page`] trait, and optionally the [`DynamicRoute`] trait. Then, pages can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
use crate::assets::PageAssets;
use crate::content::Content;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

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

/// Allows to access the content source in a [`DynamicRoute`] implementation.
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
/// impl DynamicRoute<ArticleParams> for Article {
///     fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<ArticleParams> {
///         let articles = ctx.content.get_source::<ArticleContent>("articles");
///
///         articles.into_params(|entry| ArticleParams {
///             article: entry.id.clone(),
///         })
///     }
/// }
///
/// impl Page for Article {
///    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///      let params = ctx.params::<ArticleParams>();
///      let articles = ctx.content.get_source::<ArticleContent>("articles");
///      let article = articles.get_entry(&params.article);
///      article.render().into()
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
pub trait Page<T = RenderResult>
where
    T: Into<RenderResult>,
{
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

/// Must be implemented for every dynamic route of your website.
///
/// Dynamic route allows creating many pages that share the same structure and logic, but with different content. Typically used for a [`ContentSource`](crate::content::ContentSource).
///
/// ## Example
/// ```rust
/// use maudit::page::prelude::*;
///
/// #[route("/tags/[id]")]
/// pub struct Tags;
///
/// #[derive(Params)]
/// struct Params {
///   id: String,
/// }
///
/// impl DynamicRoute for Tags {
///    fn routes(&self, context: &mut DynamicRouteContext) -> Vec<RouteParams> {
///      let tags = vec!["rust", "web", "programming"].iter().map(|tag| Params { id: tag.to_string() }).collect();
///      RouteParams::from_vec(tags)
///    }
/// }
///
/// impl Page for Tags {
///   fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///     let tag = ctx.params::<Params>().id;
///     format!("<h1>Tag: {}</h1>", tag).into()
///   }
/// }
/// ```
pub trait DynamicRoute<P = RouteParams>
where
    P: Into<RouteParams>,
{
    // Intentionally does not have a default implementation even though it'd be useful in our macros in order to force
    // the user to implement it explicitly, even if it's just returning an empty Vec.
    fn routes(&self, context: &mut DynamicRouteContext) -> Vec<P>;
}

#[doc(hidden)]
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
    fn route_type(&self) -> RouteType;
    fn route_raw(&self) -> String;
    fn route(&self, params: &RouteParams) -> String;
    fn file_path(&self, params: &RouteParams) -> PathBuf;
    fn url_unsafe<P: Into<RouteParams>>(params: P) -> String
    where
        Self: Sized;
    fn url_untyped(&self, params: &RouteParams) -> String;
}

#[doc(hidden)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::route`] implements it for the user behind the scenes.
pub trait FullPage: InternalPage + Sync {
    fn render_internal(&self, ctx: &mut RouteContext) -> RenderResult;
    fn routes_internal(&self, context: &mut DynamicRouteContext) -> Vec<RouteParams>;
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
        DynamicRoute, DynamicRouteContext, Page, RenderResult, RouteContext, RouteParams,
    };
    #[doc(hidden)]
    pub use super::{FullPage, InternalPage};
    pub use crate::assets::Asset;
    pub use crate::content::MarkdownContent;
    pub use maudit_macros::{route, Params};
}
