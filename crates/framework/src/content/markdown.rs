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
}

#[derive(Debug)]
struct InternalHeadingEvent {
    start: usize,
    end: usize,
    id: Option<String>,
    level: u32,
    classes: Vec<String>,
}

impl InternalHeadingEvent {
    fn new(start: usize, level: u32, id: Option<String>, classes: &[String]) -> Self {
        Self {
            start,
            end: 0,
            id,
            level,
            classes: classes.to_vec(),
        }
    }
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

        let mut content_events = Vec::new();
        for (event, _) in Parser::new_ext(&content, options).into_offset_iter() {
            match event {
                Event::Start(Tag::MetadataBlock(_)) => in_frontmatter = true,
                Event::End(TagEnd::MetadataBlock(_)) => in_frontmatter = false,
                Event::Text(ref text) => {
                    if in_frontmatter {
                        frontmatter.push_str(text);
                    } else {
                        content_events.push(event);
                    }
                }
                _ => content_events.push(event),
            }
        }

        // TODO: Prettier errors for serialization errors (e.g. missing fields)
        let mut parsed = serde_yml::from_str::<T>(&frontmatter).unwrap();

        let headings_internal = find_headings(&content_events);

        let mut headings = vec![];
        for heading in headings_internal {
            let heading_content = get_text_from_events(&content_events[heading.start..heading.end]);
            let slug: String = slugger.slugify(&heading_content);

            headings.push(MarkdownHeading {
                title: heading_content,
                id: heading.id.unwrap_or(slug),
                level: heading.level as u8,
                classes: heading.classes,
            });
        }

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

fn get_text_from_events(parser_slice: &[Event]) -> String {
    let mut title = String::new();

    for event in parser_slice.iter() {
        match event {
            Event::Text(text) | Event::Code(text) => title += text,
            _ => continue,
        }
    }

    title
}

fn find_headings(events: &[Event]) -> Vec<InternalHeadingEvent> {
    let mut heading_refs = vec![];

    for (i, event) in events.iter().enumerate() {
        match event {
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                heading_refs.push(InternalHeadingEvent::new(
                    i,
                    *level as u32,
                    id.clone().map(String::from),
                    &classes
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<String>>(),
                ));
            }
            Event::End(TagEnd::Heading { .. }) => {
                heading_refs
                    .last_mut()
                    .expect("Heading end before start?")
                    .end = i;
            }
            _ => (),
        }
    }

    heading_refs
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
    let mut slugger = slugger::Slugger::new();
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

    let headings = find_headings(&events);

    for heading in &headings {
        let heading_content = get_text_from_events(&events[heading.start..heading.end]);
        let slug: String = slugger.slugify(&heading_content);

        events[heading.start] = Event::Html(
            format!(
                "<h{} id=\"{}\" class=\"{}\">",
                heading.level,
                heading.id.clone().unwrap_or(slug),
                heading.classes.join(" ")
            )
            .into(),
        );
    }

    pulldown_cmark::html::push_html(&mut html_output, events.into_iter());

    html_output
}
