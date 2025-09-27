//! Core traits and structs to define the pages of your website.
//!
//! Every route must implement the [`Route`] trait. Then, pages can be passed to [`coronate()`](crate::coronate), through the [`routes!`](crate::routes) macro, to be built.
use crate::assets::{Asset, RouteAssets};
use crate::content::{Entry, RouteContent};
use crate::errors::BuildError;
use crate::routing::{
    extract_params_from_raw_route, get_route_type_from_route_params, guess_if_route_is_endpoint,
};
use rustc_hash::FxHashMap;
use std::any::Any;
use std::path::{Path, PathBuf};

use lol_html::{RewriteStrSettings, element, rewrite_str};

/// The result of a page render, can be either text, raw bytes, or an error.
///
/// Typically used through the [`Into<RenderResult>`](std::convert::Into) and [`From<RenderResult>`](std::convert::From) implementations for common types.
/// End users should rarely need to interact with this enum directly.
///
/// ## Example
/// ```rs
/// use maudit::route::prelude::*;
///
/// #[route("/")]
/// pub struct Index;
///
/// impl Route for Index {
///   fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
///    "<h1>Hello, world!</h1>"
///   }
/// }
/// ```
pub enum RenderResult {
    Text(String),
    Raw(Vec<u8>),
    Err(Box<dyn std::error::Error>),
}

impl<T> From<Result<T, Box<dyn std::error::Error>>> for RenderResult
where
    T: Into<RenderResult>,
{
    fn from(val: Result<T, Box<dyn std::error::Error>>) -> Self {
        match val {
            Ok(s) => s.into(),
            Err(e) => RenderResult::Err(e),
        }
    }
}

impl From<RenderResult> for Result<RenderResult, Box<dyn std::error::Error>> {
    fn from(val: RenderResult) -> Self {
        match val {
            RenderResult::Err(e) => Err(e),
            _ => Ok(val),
        }
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
#[derive(Clone)]
pub struct PaginationPage<T> {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_prev: bool,
    pub start_index: usize,
    pub end_index: usize,
    pub items: Vec<T>,
}

impl<T> PaginationPage<T> {
    pub fn new(page: usize, per_page: usize, total_items: usize, page_items: Vec<T>) -> Self {
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
            start_index,
            end_index,
            items: page_items,
        }
    }
}

impl<T> std::fmt::Debug for PaginationPage<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaginationPage")
            .field("page", &self.page)
            .field("per_page", &self.per_page)
            .field("total_items", &self.total_items)
            .field("total_pages", &self.total_pages)
            .field("has_next", &self.has_next)
            .field("has_prev", &self.has_prev)
            .field("start_index", &self.start_index)
            .field("end_index", &self.end_index)
            // I don't really want to force users to implement Debug for T, so just show the length of items
            .field("items", &format!("[{} items]", self.items.len()))
            .finish()
    }
}

/// Type alias for pagination pages of content entries, for easier usage
pub type PaginatedContentPage<T> = PaginationPage<Entry<T>>;

/// Helper function to create paginated routes from any iterator
///
/// Example:
/// ```rs
/// use maudit::route::prelude::*;
///
/// #[route("/tags/[page]")]
/// pub struct Tags;
///
/// #[derive(Params)]
/// pub struct TagsParams {
///     pub page: usize,
/// }
///
/// impl Route<TagsParams, PaginationPage<String>> for Tags {
///     fn pages(&self, ctx: &mut DynamicRouteContext) -> Vec<Page<TagsParams, PaginationPage<String>>> {
///         let tags = vec!["rust".to_string(), "javascript".to_string(), "python".to_string()];
///         paginate(tags, 2, |page| TagsParams { page })
///     }
///
///     fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
///         let props = ctx.props::<PaginationPage<String>>();
///         format!("Page {} of tags: {:?}", props.page + 1, props.items)
///     }
/// }
/// ```
pub fn paginate<T, I, Params>(
    items: I,
    per_page: usize,
    mut params_fn: impl FnMut(usize) -> Params,
) -> Pages<Params, PaginationPage<T>>
where
    I: IntoIterator<Item = T>,
    Params: Into<PageParams>,
    T: Clone,
{
    let items: Vec<T> = items.into_iter().collect();

    if items.is_empty() {
        return vec![];
    }

    let total_items = items.len();
    let total_pages = total_items.div_ceil(per_page);
    let mut routes = Vec::new();

    for page in 0..total_pages {
        let params = params_fn(page);

        // Calculate the slice for this specific page
        let start_index = page * per_page;
        let end_index = ((page + 1) * per_page).min(total_items);
        let page_items = items[start_index..end_index].to_vec();

        let props = PaginationPage::new(page, per_page, total_items, page_items);

        routes.push(Page::new(params, props));
    }

    routes
}

/// Allows to access various data and assets in a [`Route`] implementation.
///
/// ## Example
/// ```rs
/// use maudit::route::prelude::*;
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
///   fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
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
    /// The current path being rendered, e.g. `/articles/my-article`.
    pub current_path: &'a String,
    /// The base URL as defined in [`BuildOptions::base_url`](crate::BuildOptions::base_url)
    pub base_url: &'a Option<String>,
}

impl<'a> PageContext<'a> {
    pub fn from_static_route(
        content: &'a RouteContent,
        assets: &'a mut RouteAssets,
        current_path: &'a String,
        base_url: &'a Option<String>,
    ) -> Self {
        Self {
            params: &(),
            props: &(),
            content,
            assets,
            current_path,
            base_url,
        }
    }

    pub fn from_dynamic_route(
        dynamic_page: &'a PagesResult,
        content: &'a RouteContent,
        assets: &'a mut RouteAssets,
        current_path: &'a String,
        base_url: &'a Option<String>,
    ) -> Self {
        Self {
            params: dynamic_page.1.as_ref(),
            props: dynamic_page.2.as_ref(),
            content,
            assets,
            current_path,
            base_url,
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

    /// Returns the canonical URL for the current page. If [`BuildOptions::base_url`](crate::BuildOptions::base_url) is not set, this will return `None`.
    pub fn canonical_url(&self) -> Option<String> {
        self.base_url
            .as_ref()
            .map(|base| format!("{}{}", base, self.current_path))
    }
}

/// Allows to access the content source in the [`Page::pages`] method.
///
/// ## Example
/// ```rs
/// use maudit::route::prelude::*;
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
///    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
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
/// use maudit::route::prelude::*;
///
/// #[route("/")]
/// pub struct Index;
///
/// impl Route for Index {
///    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
///       "<h1>Hello, world!</h1>".into()
///   }
/// }
/// ```
pub trait Route<Params = PageParams, Props = ()>
where
    Params: Into<PageParams>,
    Props: 'static,
{
    fn pages(&self, _ctx: &mut DynamicRouteContext) -> Pages<Params, Props> {
        Vec::new()
    }
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult>;
}

/// Raw representation of the parameters passed to a page.
///
/// Can be accessed through [`PageContext`]'s `raw_params`.
#[derive(Clone, Default, Debug)]
pub struct PageParams(pub FxHashMap<String, Option<String>>);

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
        let params_def = extract_params_from_raw_route(&self.route_raw());
        build_url_with_params(&self.route_raw(), &params_def, params)
    }

    fn file_path(&self, params: &PageParams, output_dir: &Path) -> PathBuf {
        let params_def = extract_params_from_raw_route(&self.route_raw());
        build_file_path_with_params(
            &self.route_raw(),
            &params_def,
            params,
            output_dir,
            self.is_endpoint(),
        )
    }
}

/// Extension trait providing generic convenience methods on an instance of a route
pub trait RouteExt<Params = PageParams, Props = ()>: Route<Params, Props> + InternalRoute
where
    Params: Into<PageParams>,
    Props: 'static,
{
    /// Get the URL for this page with the given parameters
    ///
    /// Note that this method merely generates the URL based on the route pattern and parameters, it does not verify if a corresponding route actually exists.
    fn url(&self, params: Params) -> String {
        InternalRoute::url(self, &params.into())
    }
}

// Blanket implementation for all Page implementors that also implement InternalPage
impl<U, Params, Props> RouteExt<Params, Props> for U
where
    U: Route<Params, Props> + InternalRoute,
    Params: Into<PageParams>,
    Props: 'static,
{
}

/// Internal trait implemented by all routes, used by Maudit to render pages.
/// [`maudit_macros::route`] implements it automatically for the user.
pub trait FullRoute: InternalRoute + Sync + Send {
    #[doc(hidden)]
    fn render_internal(
        &self,
        ctx: &mut PageContext,
    ) -> Result<RenderResult, Box<dyn std::error::Error>>;
    #[doc(hidden)]
    fn pages_internal(&self, context: &mut DynamicRouteContext) -> PagesResults;

    fn get_pages(&self, context: &mut DynamicRouteContext) -> PagesResults {
        self.pages_internal(context)
    }

    fn build(&self, ctx: &mut PageContext) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let result = self.render_internal(ctx)?;
        let bytes = finish_route(result, ctx.assets, self.route_raw())?;

        Ok(bytes)
    }
}

use crate::routing::ParameterDef;
use std::sync::OnceLock;

// Helper functions for URL and path generation that work with pre-extracted parameters
fn build_url_with_params(
    route_template: &str,
    params_def: &[ParameterDef],
    params: &PageParams,
) -> String {
    if params_def.is_empty() {
        return route_template.to_string();
    }

    let mut result = route_template.to_string();

    for param_def in params_def {
        let value = params.0.get(&param_def.key).unwrap_or_else(|| {
            panic!(
                "Route {:?} is missing parameter {:?}",
                route_template, param_def.key
            )
        });

        let replacement = value.as_deref().unwrap_or("");
        result.replace_range(
            param_def.index..param_def.index + param_def.length,
            replacement,
        );
    }

    result.replace("//", "/")
}

fn build_file_path_with_params(
    route_template: &str,
    params_def: &[ParameterDef],
    params: &PageParams,
    output_dir: &Path,
    is_endpoint: bool,
) -> PathBuf {
    let route = if params_def.is_empty() {
        // No parameters, use template directly
        route_template
    } else {
        // Build route with reverse-ordered parameters (avoiding clone + sort)
        let mut route = route_template.to_string();

        for param_def in params_def {
            let value = params.0.get(&param_def.key).unwrap_or_else(|| {
                panic!(
                    "Route {:?} is missing parameter {:?}",
                    route_template, param_def.key
                )
            });

            let replacement = value.as_deref().unwrap_or("");
            route.replace_range(
                param_def.index..param_def.index + param_def.length,
                replacement,
            );
        }

        return build_path_from_route(&route, output_dir, is_endpoint);
    };

    build_path_from_route(route, output_dir, is_endpoint)
}

// Helper to build PathBuf from route string
fn build_path_from_route(route: &str, output_dir: &Path, is_endpoint: bool) -> PathBuf {
    // Collect all path components at once
    let parts: Vec<&str> = route.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        // Root route case
        let mut path = PathBuf::from(output_dir);
        if !is_endpoint {
            path.push("index.html");
        }
        return path;
    }

    // Build the complete path efficiently
    let mut path = PathBuf::with_capacity(output_dir.as_os_str().len() + route.len() + 20);
    path.push(output_dir);
    path.extend(&parts);

    if !is_endpoint {
        path.push("index.html");
    }

    path
}

/// Wrapper around a route that caches its parameter extraction and endpoint status to avoid redundant computations.
pub struct CachedRoute<'a> {
    inner: &'a dyn FullRoute,
    params_cache: OnceLock<Vec<ParameterDef>>,
    is_endpoint: OnceLock<bool>,
}

impl<'a> CachedRoute<'a> {
    pub fn new(route: &'a dyn FullRoute) -> Self {
        Self {
            inner: route,
            params_cache: OnceLock::new(),
            is_endpoint: OnceLock::new(),
        }
    }

    fn get_cached_params(&self) -> &Vec<ParameterDef> {
        self.params_cache
            .get_or_init(|| extract_params_from_raw_route(&self.inner.route_raw()))
    }

    fn is_endpoint(&self) -> bool {
        *self
            .is_endpoint
            .get_or_init(|| guess_if_route_is_endpoint(&self.inner.route_raw()))
    }
}

impl<'a> InternalRoute for CachedRoute<'a> {
    fn route_raw(&self) -> String {
        self.inner.route_raw()
    }

    fn route_type(&self) -> RouteType {
        get_route_type_from_route_params(self.get_cached_params())
    }

    fn url(&self, params: &PageParams) -> String {
        build_url_with_params(&self.route_raw(), self.get_cached_params(), params)
    }

    fn file_path(&self, params: &PageParams, output_dir: &Path) -> PathBuf {
        build_file_path_with_params(
            &self.route_raw(),
            self.get_cached_params(),
            params,
            output_dir,
            self.is_endpoint(),
        )
    }
}

pub fn finish_route(
    render_result: RenderResult,
    page_assets: &RouteAssets,
    route: String,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match render_result {
        // We've handled errors already at this point, but just in case, handle them again here
        RenderResult::Err(e) => Err(e),
        RenderResult::Text(html) => {
            let included_styles: Vec<_> = page_assets.included_styles().collect();
            let included_scripts: Vec<_> = page_assets.included_scripts().collect();

            if included_scripts.is_empty() && included_styles.is_empty() {
                return Ok(html.into_bytes());
            }

            let element_content_handlers = vec![
                // Add included scripts and styles to the head
                element!("head", |el| {
                    for style in &included_styles {
                        el.append(
                            &format!("<link rel=\"stylesheet\" href=\"{}\">", style.url()),
                            lol_html::html_content::ContentType::Html,
                        );
                    }

                    for script in &included_scripts {
                        el.append(
                            &format!("<script src=\"{}\" type=\"module\"></script>", script.url()),
                            lol_html::html_content::ContentType::Html,
                        );
                    }

                    Ok(())
                }),
            ];

            let output = rewrite_str(
                &html,
                RewriteStrSettings {
                    element_content_handlers,
                    ..RewriteStrSettings::new()
                },
            )?;

            Ok(output.into_bytes())
        }
        RenderResult::Raw(content) => {
            let included_styles: Vec<_> = page_assets.included_styles().collect();
            let included_scripts: Vec<_> = page_assets.included_scripts().collect();

            if !included_scripts.is_empty() || !included_styles.is_empty() {
                Err(BuildError::InvalidRenderResult { route })?;
            }

            Ok(content)
        }
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
    //! use maudit::route::prelude::*;
    //! ```
    pub use super::{
        CachedRoute, DynamicRouteContext, FullRoute, Page, PageContext, PageParams, Pages,
        PaginatedContentPage, PaginationPage, RenderResult, Route, RouteExt, paginate,
    };
    pub use crate::assets::{Asset, Image, ImageFormat, ImageOptions, Script, Style, StyleOptions};
    pub use crate::content::{
        ContentContext, ContentEntry, Entry, EntryInner, MarkdownContent, RouteContent,
    };
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
        params.insert("slug".to_string(), Some("hello-world".to_string()));
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/articles/hello-world");
    }

    #[test]
    fn test_url_multiple_parameters() {
        let page = TestPage {
            route: "/articles/tags/[tag]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("tag".to_string(), Some("rust".to_string()));
        params.insert("page".to_string(), Some("2".to_string()));
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
        params.insert(
            "tag".to_string(),
            Some("development-experience".to_string()),
        ); // Long replacement
        params.insert("page".to_string(), Some("1".to_string())); // Short replacement
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
        params.insert("lang".to_string(), Some("en".to_string()));
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/en/about");
    }

    #[test]
    fn test_url_parameter_at_end() {
        let page = TestPage {
            route: "/api/users/[id]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("id".to_string(), Some("123".to_string()));
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/api/users/123");
    }

    #[test]
    fn test_file_path_single_parameter_non_endpoint() {
        let page = TestPage {
            route: "/articles/[slug]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), Some("hello-world".to_string()));
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
        params.insert("tag".to_string(), Some("rust".to_string()));
        params.insert("page".to_string(), Some("2".to_string()));
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
        params.insert("page".to_string(), Some("1".to_string()));
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");

        // This should panic because we're missing the "tag" parameter
        page.file_path(&route_params, output_dir);
    }

    #[test]
    fn test_paginate_generic_function() {
        // Test with simple strings
        let tags = vec!["rust", "javascript", "python", "go", "typescript"];

        let routes = paginate(&tags, 2, |page| {
            let mut params = FxHashMap::default();
            params.insert("page".to_string(), Some(page.to_string()));
            PageParams(params)
        });

        assert_eq!(routes.len(), 3);

        // First page
        assert_eq!(routes[0].props.page, 0);
        assert_eq!(routes[0].props.items.len(), 2);
        assert_eq!(routes[0].props.items[0], &"rust");
        assert_eq!(routes[0].props.items[1], &"javascript");

        // Second page
        assert_eq!(routes[1].props.page, 1);
        assert_eq!(routes[1].props.items.len(), 2);
        assert_eq!(routes[1].props.items[0], &"python");
        assert_eq!(routes[1].props.items[1], &"go");

        // Third page
        assert_eq!(routes[2].props.page, 2);
        assert_eq!(routes[2].props.items.len(), 1);
        assert_eq!(routes[2].props.items[0], &"typescript");
    }

    #[test]
    fn test_url_optional_parameter_with_value() {
        let page = TestPage {
            route: "/articles/[slug]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), Some("hello-world".to_string()));
        params.insert("page".to_string(), Some("2".to_string()));
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/articles/hello-world/2");
    }

    #[test]
    fn test_url_optional_parameter_none() {
        let page = TestPage {
            route: "/articles/[slug]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), Some("hello-world".to_string()));
        params.insert("page".to_string(), None);
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/articles/hello-world/");
    }

    #[test]
    fn test_url_multiple_optional_parameters() {
        let page = TestPage {
            route: "/[lang]/articles/[category]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("lang".to_string(), None);
        params.insert("category".to_string(), Some("rust".to_string()));
        params.insert("page".to_string(), None);
        let route_params = PageParams(params);

        assert_eq!(page.url(&route_params), "/articles/rust/");
    }

    #[test]
    fn test_file_path_optional_parameter_with_value() {
        let page = TestPage {
            route: "/articles/[slug]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), Some("hello-world".to_string()));
        params.insert("page".to_string(), Some("2".to_string()));
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/articles/hello-world/2/index.html");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }

    #[test]
    fn test_file_path_optional_parameter_none() {
        let page = TestPage {
            route: "/articles/[slug]/[page]".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("slug".to_string(), Some("hello-world".to_string()));
        params.insert("page".to_string(), None);
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/articles/hello-world/index.html");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }

    #[test]
    fn test_file_path_optional_parameter_endpoint() {
        let page = TestPage {
            route: "/api/[version]/data.json".to_string(),
        };

        let mut params = FxHashMap::default();
        params.insert("version".to_string(), None);
        let route_params = PageParams(params);

        let output_dir = Path::new("/dist");
        let expected = Path::new("/dist/api/data.json");

        assert_eq!(page.file_path(&route_params, output_dir), expected);
    }
}
