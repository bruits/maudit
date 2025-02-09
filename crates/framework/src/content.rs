//! Core functions and structs to define the content sources of your website.
//!
//! Content sources represent the content of your website, such as articles, blog posts, etc. Then, content sources can be passed to [`coronate()`](crate::coronate), through the [`content_sources!`](crate::content_sources) macro, to be loaded. Typically used in [`DynamicRoute`](crate::page::DynamicRoute).
use std::{any::Any, path::PathBuf};

use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;

use crate::page::RouteParams;

/// Helps implement a struct as a Markdown content entry.
///
/// ## Example
/// ```rust
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
///       "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
///
/// ## Expand
/// ```rust
/// use maudit::content::{markdown_entry};
///
/// #[markdown_entry]
/// pub struct Article {
///   pub title: String,
///   pub content: String,
/// }
/// ```
/// expands to
/// ```rust
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
/// ```rust
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
///       "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
///
/// In a page:
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
pub struct Content<'a> {
    sources: &'a [Box<dyn ContentSourceInternal>],
}

impl Content<'_> {
    pub fn new(sources: &[Box<dyn ContentSourceInternal>]) -> Content {
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
///      article.render().into()
///   }
/// }
/// ```
pub struct ContentEntry<T> {
    pub id: String,
    render: Box<dyn Fn(&str) -> String + Send + Sync>,
    pub raw_content: String,
    pub data: T,
    pub file_path: Option<PathBuf>,
}

impl<T> ContentEntry<T> {
    pub fn render(&self) -> String {
        (self.render)(&self.raw_content)
    }
}

/// Represents an untyped content source.
pub type Untyped = FxHashMap<String, String>;

/// Represents a collection of content sources.
///
/// Mostly seen as the return type of [`content_sources!`](crate::content_sources).
///
/// ## Example
/// ```rust
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
///   content_sources!["docs" => glob_markdown::<ArticleContent>("content/docs/*.md")]
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

/// Represents a Markdown heading.
///
/// Can be used to generate a table of contents.
///
/// ## Example
/// ```rust
/// use maudit::page::prelude::*;
/// use maud::{html, Markup};
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
/// impl Page<Markup> for Article {
///   fn render(&self, ctx: &mut RouteContext) -> Markup {
///     let articles = ctx.content.get_source::<ArticleContent>("articles");
///     let article = articles.get_entry("my-article");
///     let headings = article.data.get_headings(); // returns a Vec<MarkdownHeading>
///     let toc = html! {
///       ul {
///         @for heading in headings {
///           li {
///             a href=(format!("#{}", heading.id)) { (heading.title) }
///           }
///         }
///       }
///     };
///     html! {
///       main {
///         h1 { (article.data.title) }
///         nav { (toc) }
///       }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MarkdownHeading {
    pub title: String,
    pub id: String,
    pub level: u8,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, Option<String>)>,
}

#[doc(hidden)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::markdown_entry`] implements it for the user behind the scenes.
pub trait MarkdownContent {
    fn get_headings(&self) -> &Vec<MarkdownHeading>;
}

#[doc(hidden)]
/// Used internally by Maudit and should not be implemented by the user.
/// We expose it because [`maudit_macros::markdown_entry`] implements it for the user behind the scenes.
pub trait InternalMarkdownContent {
    fn set_headings(&mut self, headings: Vec<MarkdownHeading>);
}

/// Represents untyped Markdown content.
///
/// Assumes that the Markdown content has no frontmatter.
///
/// ## Example
/// ```rust
/// use maudit::{coronate, content_sources, routes, BuildOptions, BuildOutput};
/// use maudit::content::{glob_markdown, UntypedMarkdownContent};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![
///       "articles" => glob_markdown::<UntypedMarkdownContent>("content/spooky/*.md")
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
#[derive(serde::Deserialize)]
pub struct UntypedMarkdownContent {
    #[serde(skip)]
    __internal_headings: Vec<MarkdownHeading>,
}

impl MarkdownContent for UntypedMarkdownContent {
    fn get_headings(&self) -> &Vec<MarkdownHeading> {
        &self.__internal_headings
    }
}

impl InternalMarkdownContent for UntypedMarkdownContent {
    fn set_headings(&mut self, headings: Vec<MarkdownHeading>) {
        self.__internal_headings = headings;
    }
}

/// Glob for Markdown files and return a vector of [`ContentEntry`]s.
///
/// Typically used by [`content_sources!`](crate::content_sources) to define a Markdown content source in [`coronate()`](crate::coronate).
///
/// ## Example
/// ```rust
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
///       "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
pub fn glob_markdown<T>(pattern: &str) -> Vec<ContentEntry<T>>
where
    T: DeserializeOwned + MarkdownContent + InternalMarkdownContent,
{
    let mut entries = vec![];

    for entry in glob_fs(pattern).unwrap() {
        let entry = entry.unwrap();

        if let Some(extension) = entry.extension() {
            if extension != "md" {
                warn!("Other file types than Markdown are not supported yet");
                continue;
            }
        }

        let id = entry.file_stem().unwrap().to_str().unwrap().to_string();
        let content = std::fs::read_to_string(&entry).unwrap();

        let mut options = Options::empty();
        options.insert(
            Options::ENABLE_YAML_STYLE_METADATA_BLOCKS | Options::ENABLE_HEADING_ATTRIBUTES,
        );

        let mut frontmatter = String::new();
        let mut in_frontmatter = false;

        let mut headings: Vec<MarkdownHeading> = vec![];
        let mut last_heading: Option<MarkdownHeading> = Option::None;

        for (event, _) in Parser::new_ext(&content, options).into_offset_iter() {
            match event {
                Event::Start(Tag::MetadataBlock(_)) => in_frontmatter = true,
                Event::End(TagEnd::MetadataBlock(_)) => in_frontmatter = false,
                Event::Text(ref text) => {
                    if in_frontmatter {
                        frontmatter.push_str(text);
                    }

                    // TODO: Take the entire content, not just the text
                    if let Some(ref mut heading) = last_heading {
                        heading.title.push_str(text);
                    }
                }
                Event::Start(Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs,
                }) => {
                    if !in_frontmatter {
                        last_heading = Some(MarkdownHeading {
                            title: String::new(),
                            id: if let Some(id) = id {
                                id.to_string()
                            } else {
                                // TODO: Generate an ID from the title
                                String::new()
                            },
                            level: level as u8,
                            classes: classes.iter().map(|c| c.to_string()).collect(),
                            attrs: attrs
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.as_ref().map(|v| v.to_string())))
                                .collect(),
                        });
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(heading) = last_heading.take() {
                        headings.push(heading);
                    }
                }
                _ => {}
            }
        }

        let mut parsed = serde_yml::from_str::<T>(&frontmatter).unwrap();

        parsed.set_headings(headings);

        entries.push(ContentEntry {
            id,
            render: Box::new(render_markdown),
            raw_content: content,
            file_path: Some(entry),
            data: parsed,
        });
    }

    entries
}

/// Render Markdown content to HTML.
///
/// ## Example
/// ```rust
/// use maudit::content::render_markdown;
/// let markdown = r#"# Hello, world!"#;
/// let html = render_markdown(markdown);
/// ```
pub fn render_markdown(content: &str) -> String {
    let mut html_output = String::new();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let mut in_frontmatter = false;
    let mut events = Vec::new();
    for (event, _) in Parser::new_ext(content, options).into_offset_iter() {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                in_frontmatter = true;
            }
            Event::End(TagEnd::MetadataBlock(_)) => {
                in_frontmatter = false;
            }
            Event::Text(_) => {
                if !in_frontmatter {
                    events.push(event);
                }
            }
            _ => {
                events.push(event);
            }
        }
    }

    pulldown_cmark::html::push_html(&mut html_output, events.into_iter());

    html_output
}
