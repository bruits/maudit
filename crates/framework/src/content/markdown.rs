use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::de::DeserializeOwned;

use super::{slugger, ContentEntry};

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
/// impl Page<RouteParams, Markup> for Article {
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

#[derive(Debug)]
struct InternalHeadingEvent {
    start: usize,
    end: usize,
    id: Option<String>,
    level: u32,
    classes: Vec<String>,
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
#[derive(serde::Deserialize, Debug, Clone)]
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
        let mut slugger = slugger::Slugger::new();
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
                                slugger.slugify(&content)
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
            render: Some(Box::new(render_markdown)),
            raw_content: Some(content),
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
