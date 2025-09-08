//! Core functions and structs to define the content sources of your website.
//!
//! Content sources represent the content of your website, such as articles, blog posts, etc. Then, content sources can be passed to [`coronate()`](crate::coronate), through the [`content_sources!`](crate::content_sources) macro, to be loaded.
use std::{any::Any, path::PathBuf};

use rustc_hash::FxHashMap;

mod highlight;
pub mod markdown;
mod slugger;

use crate::page::{RouteContext, RouteParams};
pub use markdown::{
    components::{
        BlockQuoteKind, BlockquoteComponent, CodeComponent, EmphasisComponent, HardBreakComponent,
        HeadingComponent, HorizontalRuleComponent, ImageComponent, LinkComponent, LinkType,
        ListComponent, ListItemComponent, ListType, MarkdownComponents, ParagraphComponent,
        StrikethroughComponent, StrongComponent, TableAlignment, TableCellComponent,
        TableComponent, TableHeadComponent, TableRowComponent, TaskListMarkerComponent,
    },
    *,
};

/// Helps implement a struct as a Markdown content entry.
///
/// ## Example
/// ```rs
/// use maudit::{coronate, content_sources, routes, BuildOptions, BuildOutput};
/// use maudit::content::{markdown_entry, glob_markdown};
///
/// #[markdown_entry]
/// pub struct ArticleContent {
///   pub title: String,
///   pub description: String,
/// }
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![
///       "articles" => glob_markdown::<ArticleContent>("content/articles/*.md", None)
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
///
/// ## Expand
/// ```rs
/// use maudit::content::{markdown_entry};
///
/// #[markdown_entry]
/// pub struct Article {
///   pub title: String,
///   pub content: String,
/// }
/// ```
/// expands to
/// ```rs
/// #[derive(serde::Deserialize)]
/// pub struct Article {
///   pub title: String,
///   pub content: String,
///   #[serde(skip)]
///   __internal_headings: Vec<maudit::content::MarkdownHeading>
/// }
///
/// impl maudit::content::MarkdownContent for Article {
///   fn get_headings(&self) -> &Vec<maudit::content::MarkdownHeading> {
///     &self.__internal_headings
///   }
/// }
///
/// impl maudit::content::InternalMarkdownContent for Article {
///   fn set_headings(&mut self, headings: Vec<maudit::content::MarkdownHeading>) {
///     self.__internal_headings = headings;
///   }
/// }
/// ```
pub use maudit_macros::markdown_entry;

/// Main struct to access all content sources.
///
/// Can only access content sources that have been defined in [`coronate()`](crate::coronate).
///
/// ## Example
/// In `main.rs`:
/// ```rs
/// use maudit::{coronate, content_sources, routes, BuildOptions, BuildOutput};
/// use maudit::content::{markdown_entry, glob_markdown};
///
/// #[markdown_entry]
/// pub struct ArticleContent {
///   pub title: String,
///   pub description: String,
/// }
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![
///       "articles" => glob_markdown::<ArticleContent>("content/articles/*.md", None)
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
///
/// In a page:
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
///      article.render(ctx).into()
///   }
///
///   fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<ArticleParams> {
///     let articles = ctx.content.get_source::<ArticleContent>("articles");
///
///     articles.into_params(|entry| ArticleParams {
///       article: entry.id.clone(),
///     })
///   }
/// }
/// ```
pub struct Content<'a> {
    sources: &'a [Box<dyn ContentSourceInternal>],
}

impl Content<'_> {
    pub fn new(sources: &'_ [Box<dyn ContentSourceInternal>]) -> Content<'_> {
        Content { sources }
    }

    pub fn get_untyped_source(&self, name: &str) -> &ContentSource<Untyped> {
        self.sources
            .iter()
            .find_map(
                |source| match source.as_any().downcast_ref::<ContentSource<Untyped>>() {
                    Some(source) if source.name == name => Some(source),
                    _ => None,
                },
            )
            .unwrap_or_else(|| panic!("Content source with name '{}' not found", name))
    }

    pub fn get_untyped_source_safe(&self, name: &str) -> Option<&ContentSource<Untyped>> {
        self.sources.iter().find_map(|source| {
            match source.as_any().downcast_ref::<ContentSource<Untyped>>() {
                Some(source) if source.name == name => Some(source),
                _ => None,
            }
        })
    }

    pub fn get_source<T: 'static>(&self, name: &str) -> &ContentSource<T> {
        self.sources
            .iter()
            .find_map(
                |source| match source.as_any().downcast_ref::<ContentSource<T>>() {
                    Some(source) if source.name == name => Some(source),
                    _ => None,
                },
            )
            .unwrap_or_else(|| panic!("Content source with name '{}' not found", name))
    }

    pub fn get_source_safe<T: 'static>(&self, name: &str) -> Option<&ContentSource<T>> {
        self.sources.iter().find_map(|source| {
            match source.as_any().downcast_ref::<ContentSource<T>>() {
                Some(source) if source.name == name => Some(source),
                _ => None,
            }
        })
    }
}

/// Represents a single entry in a [`ContentSource`].
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
/// #[route("/articles/my-article")]
/// pub struct Article;
///
/// #[derive(Params)]
/// pub struct ArticleParams {
///     pub article: String,
/// }
///
/// impl Page for Article {
///    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
///      let articles = ctx.content.get_source::<ArticleContent>("articles");
///      let article = articles.get_entry("my-article"); // returns a ContentEntry
///      article.render(ctx).into()
///   }
/// }
/// ```
pub struct ContentEntry<T> {
    pub id: String,
    render: OptionalContentRenderFn,
    pub raw_content: Option<String>,
    data_loader: OptionalDataLoadingFn<T>,
    cached_data: std::sync::OnceLock<T>,
    pub file_path: Option<PathBuf>,
}

type OptionalDataLoadingFn<T> =
    Option<Box<dyn Fn(&mut crate::page::RouteContext) -> T + Send + Sync>>;

type OptionalContentRenderFn =
    Option<Box<dyn Fn(&str, &mut crate::page::RouteContext) -> String + Send + Sync>>;

impl<T> ContentEntry<T> {
    pub fn new(
        id: String,
        render: OptionalContentRenderFn,
        raw_content: Option<String>,
        data: T,
        file_path: Option<PathBuf>,
    ) -> Self {
        Self {
            id,
            render,
            raw_content,
            data_loader: None,
            cached_data: std::sync::OnceLock::from(data),
            file_path,
        }
    }

    pub fn new_lazy(
        id: String,
        render: OptionalContentRenderFn,
        raw_content: Option<String>,
        data_loader: Box<dyn Fn(&mut crate::page::RouteContext) -> T + Send + Sync>,
        file_path: Option<PathBuf>,
    ) -> Self {
        Self {
            id,
            render,
            raw_content,
            data_loader: Some(data_loader),
            cached_data: std::sync::OnceLock::new(),
            file_path,
        }
    }

    pub fn data(&self, ctx: &mut RouteContext) -> &T {
        self.cached_data.get_or_init(|| {
            if let Some(ref loader) = self.data_loader {
                loader(ctx)
            } else {
                panic!("No data loader available and no cached data")
            }
        })
    }

    pub fn render(&self, ctx: &mut RouteContext) -> String {
        (self.render.as_ref().unwrap())(self.raw_content.as_ref().unwrap(), ctx)
    }
}

/// Represents an untyped content source.
pub type Untyped = FxHashMap<String, String>;

/// Represents a collection of content sources.
///
/// Mostly seen as the return type of [`content_sources!`](crate::content_sources).
///
/// ## Example
/// ```rs
/// use maudit::page::prelude::*;
/// use maudit::content::{glob_markdown, ContentSources};
/// use maudit::content_sources;
/// # use maudit::content::markdown_entry;
/// #
/// # #[markdown_entry]
/// # pub struct ArticleContent {
/// #    pub title: String,
/// #    pub description: String,
/// # }
///
/// pub fn content_sources() -> ContentSources {
///   content_sources!["docs" => glob_markdown::<ArticleContent>("content/docs/*.md", None)]
/// }
pub struct ContentSources(pub Vec<Box<dyn ContentSourceInternal>>);

impl From<Vec<Box<dyn ContentSourceInternal>>> for ContentSources {
    fn from(content_sources: Vec<Box<dyn ContentSourceInternal>>) -> Self {
        Self(content_sources)
    }
}

impl ContentSources {
    pub fn new(content_sources: Vec<Box<dyn ContentSourceInternal>>) -> Self {
        Self(content_sources)
    }
}

type ContentSourceInitMethod<T> = Box<dyn Fn() -> Vec<ContentEntry<T>> + Send + Sync>;

/// A source of content such as articles, blog posts, etc.
pub struct ContentSource<T = Untyped> {
    pub name: String,
    pub entries: Vec<ContentEntry<T>>,
    pub(crate) init_method: ContentSourceInitMethod<T>,
}

impl<T> ContentSource<T> {
    pub fn new<P>(name: P, entries: ContentSourceInitMethod<T>) -> Self
    where
        P: Into<String>,
    {
        Self {
            name: name.into(),
            entries: vec![],
            init_method: entries,
        }
    }

    pub fn get_entry(&self, id: &str) -> &ContentEntry<T> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .unwrap_or_else(|| panic!("Entry with id '{}' not found", id))
    }

    pub fn get_entry_safe(&self, id: &str) -> Option<&ContentEntry<T>> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn into_params<P>(&self, cb: impl Fn(&ContentEntry<T>) -> P) -> Vec<P>
    where
        P: Into<RouteParams>,
    {
        self.entries.iter().map(cb).collect()
    }

    pub fn into_routes<Params, Props>(&self, cb: impl Fn(&ContentEntry<T>) -> crate::page::Route<Params, Props>) -> Vec<crate::page::Route<Params, Props>>
    where
        Params: Into<RouteParams>,
    {
        self.entries.iter().map(cb).collect()
    }
}

#[doc(hidden)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because it's implemented for [`ContentSource`], which is public.
pub trait ContentSourceInternal: Send + Sync {
    fn init(&mut self);
    fn get_name(&self) -> &str;
    fn as_any(&self) -> &dyn Any; // Used for type checking at runtime
}

impl<T: 'static + Sync + Send> ContentSourceInternal for ContentSource<T> {
    fn init(&mut self) {
        self.entries = (self.init_method)();
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
