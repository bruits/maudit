//! Core traits and structs to define the pages of your website.
//!
//! Every page must implement the [`Page`] trait. Then, pages can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
use crate::assets::RouteAssets;
use crate::build::finish_route;
use crate::content::RouteContent;
use crate::routing::{
    extract_params_from_raw_route, get_route_type_from_route_params, guess_if_route_is_endpoint,
};
use rustc_hash::FxHashMap;
use std::any::Any;
use std::path::{Path, PathBuf};

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
/// impl Route for Index {
///   fn render(&self, ctx: &mut PageContext) -> RenderResult {
///    "<h1>Hello, world!</h1>".into()
///   }
/// }
/// ```
pub enum RenderResult {
    Text(String),
    Raw(Vec<u8>),
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

pub type Pages<Params = PageParams, Props = ()> = Vec<Page<Params, Props>>;

/// Represents a route with its parameters and associated props
#[derive(Debug, Clone)]
pub struct Page<Params = PageParams, Props = ()>
where
    Params: Into<PageParams>,
{
    pub params: Params,
    pub props: Props,
}

impl<Params, Props> Page<Params, Props>
where
    Params: Into<PageParams>,
{
    pub fn new(params: Params, props: Props) -> Self {
        Self { params, props }
    }
}

impl<Params> Page<Params, ()>
where
    Params: Into<PageParams>,
{
    pub fn from_params(params: Params) -> Self {
        Self { params, props: () }
    }
}

/// Pagination page for any type of items
pub struct PaginationPage<'a, T> {
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
    pub items: &'a [T],
}

impl<'a, T> PaginationPage<'a, T> {
    pub fn new(page: usize, per_page: usize, total_items: usize, items: &'a [T]) -> Self {
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
            items: &items[start_index..end_index],
        }
    }
}

impl<'a, T> std::fmt::Debug for PaginationPage<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaginationPage")
            .field("page", &self.page)
            .field("per_page", &self.per_page)
            .field("total_items", &self.total_items)
            .field("total_pages", &self.total_pages)
            .field("has_next", &self.has_next)
            .field("has_prev", &self.has_prev)
            .field("next_page", &self.next_page)
            .field("prev_page", &self.prev_page)
            .field("start_index", &self.start_index)
            .field("end_index", &self.end_index)
            // I don't really want to force users to implement Debug for T, so just show the length of items
            .field("items", &format!("[{} items]", self.items.len()))
            .finish()
    }
}

/// Helper function to create paginated routes from any slice
pub fn paginate<T, Params>(
    items: &[T],
    per_page: usize,
    mut params_fn: impl FnMut(usize) -> Params,
) -> Pages<Params, PaginationPage<'_, T>>
where
    Params: Into<PageParams>,
{
    if items.is_empty() {
        return vec![];
    }

    let total_items = items.len();
    let total_pages = total_items.div_ceil(per_page);
    let mut routes = Vec::new();

    for page in 0..total_pages {
        let params = params_fn(page);
        let props = PaginationPage::new(page, per_page, total_items, items);

        routes.push(Page::new(params, props));
    }

    routes
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
/// impl Route for Index {
///   fn render(&self, ctx: &mut PageContext) -> RenderResult {
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
pub struct PageContext<'a> {
    pub params: &'a dyn Any,
    pub props: &'a dyn Any,
    pub content: &'a RouteContent<'a>,
    pub assets: &'a mut RouteAssets,
    pub current_url: &'a String,
}

impl<'a> PageContext<'a> {
    pub fn from_static_route(
        content: &'a RouteContent,
        assets: &'a mut RouteAssets,
        current_url: &'a String,
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
        dynamic_page: &'a PagesResult,
        content: &'a RouteContent,
        assets: &'a mut RouteAssets,
        current_url: &'a String,
    ) -> Self {
        Self {
            params: dynamic_page.1.as_ref(),
            props: dynamic_page.2.as_ref(),
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

/// Allows to access the content source in the [`Page::pages`] method.
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
/// impl Route<ArticleParams> for Article {
///    fn render(&self, ctx: &mut PageContext) -> RenderResult {
///      let params = ctx.params::<ArticleParams>();
///      let articles = ctx.content.get_source::<ArticleContent>("articles");
///      let article = articles.get_entry(&params.article);
///      article.render().into()
///   }
///
///    fn pages(&self, ctx: &mut DynamicRouteContext) -> Vec<ArticleParams> {
///       let articles = ctx.content.get_source::<ArticleContent>("articles");
///
///       articles.into_params(|entry| ArticleParams {
///           article: entry.id.clone(),
///       })
///   }
/// }
/// ```
pub struct DynamicRouteContext<'a> {
    pub content: &'a RouteContent<'a>,
    pub assets: &'a mut RouteAssets,
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
/// impl Route for Index {
///    fn render(&self, ctx: &mut PageContext) -> RenderResult {
///       "<h1>Hello, world!</h1>".into()
///   }
/// }
/// ```
pub trait Route<Params = PageParams, Props = (), T = RenderResult>
where
    Params: Into<PageParams>,
    Props: 'static,
    T: Into<RenderResult>,
{
    fn pages(&self, _ctx: &mut DynamicRouteContext) -> Pages<Params, Props> {
        Vec::new()
    }
    fn render(&self, ctx: &mut PageContext) -> T;
}

/// Raw representation of the parameters passed to a page.
///
/// Can be accessed through [`PageContext`]'s `raw_params`.
#[derive(Clone, Default, Debug)]
pub struct PageParams(pub FxHashMap<String, String>);

impl PageParams {
    pub fn from_vec<T>(params: Vec<T>) -> Vec<PageParams>
    where
        T: Into<PageParams>,
    {
        params.into_iter().map(|p| p.into()).collect()
    }
}

impl From<&PageParams> for PageParams {
    fn from(params: &PageParams) -> Self {
        params.clone()
    }
}

impl<T> FromIterator<T> for PageParams
where
    T: Into<PageParams>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut map = FxHashMap::default();
        for item in iter {
            let item = item.into();
            map.extend(item.0);
        }
        PageParams(map)
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
pub trait InternalRoute {
    fn route_raw(&self) -> String;
    fn is_endpoint(&self) -> bool {
        guess_if_route_is_endpoint(&self.route_raw())
    }
    fn route_type(&self) -> RouteType {
        let params_def = extract_params_from_raw_route(&self.route_raw());

        get_route_type_from_route_params(&params_def)
    }

    fn url(&self, params: &PageParams) -> String {
        let mut params_def = extract_params_from_raw_route(&self.route_raw());

        // Replace every param_def with the value from the params hashmap for said key
        // So, ex: "/articles/[article]" (params: Hashmap {article: "truc"}) -> "/articles/truc"
        let mut route = self.route_raw();

        // Sort params by index in reverse order to avoid index shifting issues
        params_def.sort_by(|a, b| b.index.cmp(&a.index));

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

    fn file_path(&self, params: &PageParams, output_dir: &Path) -> PathBuf {
        let mut params_def = extract_params_from_raw_route(&self.route_raw());
        let mut route = self.route_raw();

        // Sort params by index in reverse order to avoid index shifting issues
        params_def.sort_by(|a, b| b.index.cmp(&a.index));

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

        output_dir.join(match self.is_endpoint() {
            true => cleaned_raw_route,
            false => match cleaned_raw_route.is_empty() {
                true => "index.html".into(),
                false => format!("{}/index.html", cleaned_raw_route),
            },
        })
    }
}

/// Extension trait providing generic convenience methods on an instance of a route
pub trait RouteExt<Params = PageParams, Props = (), T = RenderResult>:
    Route<Params, Props, T> + InternalRoute
where
    Params: Into<PageParams>,
    Props: 'static,
    T: Into<RenderResult>,
{
    /// Get the URL for this page with the given parameters
    ///
    /// Note that this method merely generates the URL based on the route pattern and parameters, it does not verify if a corresponding route actually exists.
    fn url(&self, params: Params) -> String {
        InternalRoute::url(self, &params.into())
    }
}

// Blanket implementation for all Page implementors that also implement InternalPage
impl<U, Params, Props, T> RouteExt<Params, Props, T> for U
where
    U: Route<Params, Props, T> + InternalRoute,
    Params: Into<PageParams>,
    Props: 'static,
    T: Into<RenderResult>,
{
}

/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::route`] implements it for the user behind the scenes.
pub trait FullRoute: InternalRoute + Sync + Send {
    #[doc(hidden)]
    fn render_internal(&self, ctx: &mut PageContext) -> RenderResult;
    #[doc(hidden)]
    fn pages_internal(&self, context: &mut DynamicRouteContext) -> PagesResults;

    fn get_pages(&self, context: &mut DynamicRouteContext) -> PagesResults {
        self.pages_internal(context)
    }

    fn build(&self, ctx: &mut PageContext) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let result = self.render_internal(ctx);
        let bytes = finish_route(result, ctx.assets, self.route_raw())?;

        Ok(bytes)
    }
}

pub type PagesResult = (PageParams, PageProps, PageTypedParams);
pub type PagesResults = Vec<PagesResult>;

pub type PageProps = Box<dyn Any + Send + Sync>;
pub type PageTypedParams = Box<dyn Any + Send + Sync>;

pub mod prelude {
    //! Re-exports of the most commonly used types and traits for defining routes.
    //!
    //! This module is meant to be glob imported in your routes files.
    //!
    //! ## Example
    //! ```rs
    //! use maudit::page::prelude::*;
    //! ```
    pub use super::{
        DynamicRouteContext, FullRoute, Page, PageContext, PageParams, Pages, PaginationPage,
        RenderResult, Route, RouteExt, paginate,
    };
    pub use crate::assets::{Asset, Image, Style, StyleOptions};
    pub use crate::content::MarkdownContent;
    pub use maudit_macros::{Params, route};
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;
    use std::path::Path;

    // Test struct implementing InternalPage for testing
    struct TestPage {
        route: String,
    }

    impl InternalRoute for TestPage {
        fn route_raw(&self) -> String {
            self.route.clone()
        }
    }

    #[test]
    fn test_url_single_parameter() {
        let page = TestPage {
            route: "/articles/[slug]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), "hello-world".to_string());
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/articles/hello-world");
    }

    #[test]
    fn test_url_multiple_parameters() {
        let page = TestPage {
            route: "/articles/tags/[tag]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("tag".to_string(), "rust".to_string());
        params.insert("page".to_string(), "2".to_string());
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/articles/tags/rust/2");
    }

    #[test]
    fn test_url_multiple_parameters_different_lengths() {
        // This specifically tests the bug we fixed where parameter replacement
        // would create invalid indices for subsequent parameters
        let page = TestPage {
            route: "/articles/tags/[tag]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("tag".to_string(), "development-experience".to_string()); // Long replacement
        params.insert("page".to_string(), "1".to_string()); // Short replacement
        let route_params = PageParams(params);

        assert_eq!(
            page.url(&route_params),
            "/articles/tags/development-experience/1"
        );
    }

    #[test]
    fn test_url_no_parameters() {
        let page = TestPage {
            route: "/about".to_string(),
        };

        let route_params = PageParams(FxHashMap::default());

        assert_eq!(page.url(&route_params), "/about");
    }

    #[test]
    fn test_url_parameter_at_start() {
        let page = TestPage {
            route: "/[lang]/about".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("lang".to_string(), "en".to_string());
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/en/about");
    }

    #[test]
    fn test_url_parameter_at_end() {
        let page = TestPage {
            route: "/api/users/[id]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("id".to_string(), "123".to_string());
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/api/users/123");
    }

    #[test]
    fn test_file_path_single_parameter_non_endpoint() {
        let page = TestPage {
            route: "/articles/[slug]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), "hello-world".to_string());
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/articles/hello-world/index.html");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }

    #[test]
    fn test_file_path_multiple_parameters_non_endpoint() {
        let page = TestPage {
            route: "/articles/tags/[tag]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("tag".to_string(), "rust".to_string());
        params.insert("page".to_string(), "2".to_string());
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/articles/tags/rust/2/index.html");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }

    #[test]
    fn test_file_path_root_route() {
        let page = TestPage {
            route: "/".to_string(),
        };

        let route_params = PageParams(FxHashMap::default());
        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/index.html");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }

    #[test]
    fn test_file_path_endpoint() {
        let page = TestPage {
            route: "/api/data.json".to_string(),
        };

        let route_params = PageParams(FxHashMap::default());
        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/api/data.json");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }

    #[test]
    #[should_panic(expected = "Route \"/articles/[slug]\" is missing parameter \"slug\"")]
    fn test_url_missing_parameter_panics() {
        let page = TestPage {
            route: "/articles/[slug]".to_string(),
        };

        let route_params = PageParams(FxHashMap::default());

        // This should panic because we're missing the "slug" parameter
        page.url(&route_params);
    }

    #[test]
    #[should_panic(expected = "Route \"/articles/tags/[tag]/[page]\" is missing parameter \"tag\"")]
    fn test_file_path_missing_parameter_panics() {
        let page = TestPage {
            route: "/articles/tags/[tag]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("page".to_string(), "1".to_string());
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");

        // This should panic because we're missing the "tag" parameter
        page.file_path(&route_params, output_dir);
    }

    #[test]
    fn test_pagination_page_with_entries() {
        // Create some mock content entries
        use crate::content::ContentEntry;
        use std::path::PathBuf;

        let entries = vec![
            ContentEntry::new(
                "entry1".to_string(),
                None,
                Some("content1".to_string()),
                (),
                Some(PathBuf::from("file1.md")),
            ),
            ContentEntry::new(
                "entry2".to_string(),
                None,
                Some("content2".to_string()),
                (),
                Some(PathBuf::from("file2.md")),
            ),
            ContentEntry::new(
                "entry3".to_string(),
                None,
                Some("content3".to_string()),
                (),
                Some(PathBuf::from("file3.md")),
            ),
            ContentEntry::new(
                "entry4".to_string(),
                None,
                Some("content4".to_string()),
                (),
                Some(PathBuf::from("file4.md")),
            ),
            ContentEntry::new(
                "entry5".to_string(),
                None,
                Some("content5".to_string()),
                (),
                Some(PathBuf::from("file5.md")),
            ),
        ];

        let pagination = PaginationPage::new(1, 2, 5, &entries);

        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 2);
        assert_eq!(pagination.total_items, 5);
        assert_eq!(pagination.total_pages, 3);
        assert!(pagination.has_next);
        assert!(pagination.has_prev);
        assert_eq!(pagination.start_index, 2);
        assert_eq!(pagination.end_index, 4);
        assert_eq!(pagination.items.len(), 2);
        assert_eq!(pagination.items[0].id, "entry3");
        assert_eq!(pagination.items[1].id, "entry4");
    }

    #[test]
    fn test_paginate_content_function() {
        use crate::content::ContentEntry;
        use std::path::PathBuf;

        let entries = vec![
            ContentEntry::new(
                "entry1".to_string(),
                None,
                Some("content1".to_string()),
                (),
                Some(PathBuf::from("file1.md")),
            ),
            ContentEntry::new(
                "entry2".to_string(),
                None,
                Some("content2".to_string()),
                (),
                Some(PathBuf::from("file2.md")),
            ),
            ContentEntry::new(
                "entry3".to_string(),
                None,
                Some("content3".to_string()),
                (),
                Some(PathBuf::from("file3.md")),
            ),
        ];

        let routes = paginate(&entries, 2, |page| {
            let mut params = FxHashMap::default();
            params.insert("page".to_string(), page.to_string());
            PageParams(params)
        });

        assert_eq!(routes.len(), 2);

        // First page
        assert_eq!(routes[0].props.page, 0);
        assert_eq!(routes[0].props.items.len(), 2);
        assert_eq!(routes[0].props.items[0].id, "entry1");
        assert_eq!(routes[0].props.items[1].id, "entry2");

        // Second page
        assert_eq!(routes[1].props.page, 1);
        assert_eq!(routes[1].props.items.len(), 1);
        assert_eq!(routes[1].props.items[0].id, "entry3");
    }

    #[test]
    fn test_paginate_generic_function() {
        // Test with simple strings
        let tags = vec!["rust", "javascript", "python", "go", "typescript"];

        let routes = paginate(&tags, 2, |page| {
            let mut params = FxHashMap::default();
            params.insert("page".to_string(), page.to_string());
            PageParams(params)
        });

        assert_eq!(routes.len(), 3);

        // First page
        assert_eq!(routes[0].props.page, 0);
        assert_eq!(routes[0].props.items.len(), 2);
        assert_eq!(routes[0].props.items[0], "rust");
        assert_eq!(routes[0].props.items[1], "javascript");

        // Second page
        assert_eq!(routes[1].props.page, 1);
        assert_eq!(routes[1].props.items.len(), 2);
        assert_eq!(routes[1].props.items[0], "python");
        assert_eq!(routes[1].props.items[1], "go");

        // Third page
        assert_eq!(routes[2].props.page, 2);
        assert_eq!(routes[2].props.items.len(), 1);
        assert_eq!(routes[2].props.items[0], "typescript");
    }
}
