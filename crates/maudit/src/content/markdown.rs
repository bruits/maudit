use std::{path::Path, sync::Arc};

use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html::push_html};
use serde::de::DeserializeOwned;

pub mod components;
pub mod shortcodes;

use components::{LinkType, ListType, MarkdownComponents, TableAlignment};

use crate::{
    assets::Asset,
    content::{
        ContentContext,
        shortcodes::{MarkdownShortcodes, preprocess_shortcodes},
    },
    page::PageContext,
};

use super::{ContentEntry, highlight::CodeBlock, slugger};

#[cfg(test)]
mod shortcodes_tests;

/// Represents a Markdown heading.
///
/// Can be used to generate a table of contents.
///
/// ## Example
/// ```rs
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
///   fn render(&self, ctx: &mut PageContext) -> Markup {
///     let articles = ctx.content.get_source::<ArticleContent>("articles");
///     let article = articles.get_entry("my-article");
///     let headings = article.data(ctx).get_headings(); // returns a Vec<MarkdownHeading>
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
///         h1 { (article.data(ctx).title) }
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
/// ```rs
/// use maudit::{coronate, content_sources, routes, BuildOptions, BuildOutput};
/// use maudit::content::{glob_markdown, UntypedMarkdownContent};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![
///       "articles" => glob_markdown::<UntypedMarkdownContent>("content/spooky/*.md", None)
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

#[derive(Default)]
pub struct MarkdownOptions {
    pub components: MarkdownComponents,
    pub shortcodes: MarkdownShortcodes,
}

impl MarkdownOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_components(components: MarkdownComponents, shortcodes: MarkdownShortcodes) -> Self {
        Self {
            components,
            shortcodes,
        }
    }
}

/// Glob for Markdown files and return a vector of [`ContentEntry`]s.
///
/// Typically used by [`content_sources!`](crate::content_sources) to define a Markdown content source in [`coronate()`](crate::coronate).
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
pub fn glob_markdown<T>(pattern: &str, options: Option<MarkdownOptions>) -> Vec<ContentEntry<T>>
where
    T: DeserializeOwned + MarkdownContent + InternalMarkdownContent + Send + Sync + 'static,
{
    let mut entries = vec![];
    let options = options.map(Arc::new);

    for entry in glob_fs(pattern).unwrap() {
        let entry = entry.unwrap();

        if let Some(extension) = entry.extension()
            && extension != "md"
        {
            warn!("Other file types than Markdown are not supported yet");
            continue;
        }

        let id = entry.file_stem().unwrap().to_str().unwrap().to_string();
        let content = std::fs::read_to_string(&entry).unwrap();

        // Clone content for the closure
        let content_clone = content.clone();
        let data_loader = Box::new(move |_: &mut dyn ContentContext| {
            parse_markdown_with_frontmatter(&content_clone)
        });

        // Perhaps not ideal, but I don't know better. We're at the "get it working" stage - erika, 2025-08-24
        // Ideally, we'd at least avoid the allocation here whenever `options` is None, not sure how to do that ergonomically
        let opts = options.clone();
        let path = entry.clone();

        entries.push(ContentEntry::new_lazy(
            id,
            Some(Box::new(move |content: &str, route_ctx| {
                render_markdown(content, opts.as_deref(), Some(&path), Some(route_ctx))
            })),
            Some(content),
            data_loader,
            Some(entry),
        ));
    }

    entries
}

fn get_text_from_events(events_slice: &[Event]) -> String {
    events_slice.iter().fold(String::new(), |mut acc, event| {
        match event {
            Event::Text(text) | Event::Code(text) => acc.push_str(text),
            _ => {}
        }
        acc
    })
}

fn find_headings(events: &[Event]) -> Vec<InternalHeadingEvent> {
    let mut headings = Vec::new();

    for (i, event) in events.iter().enumerate() {
        match event {
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                headings.push(InternalHeadingEvent::new(
                    i,
                    *level as u32,
                    id.as_ref().map(|s| s.to_string()),
                    &classes.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
                ));
            }
            Event::End(TagEnd::Heading { .. }) => {
                if let Some(heading) = headings.last_mut() {
                    heading.end = i;
                }
            }
            _ => {}
        }
    }

    headings
}

/// Render Markdown content to HTML with optional custom components.
///
/// ## Example
/// ```rs
/// use maudit::content::{render_markdown, MarkdownOptions, MarkdownComponents};
/// use maudit::content::components::HeadingComponent;
///
/// // Without components
/// let markdown = r#"# Hello, world!"#;
/// let html = render_markdown(markdown, None);
///
/// // With components
/// struct MyCustomHeading;
/// impl HeadingComponent for MyCustomHeading {
///     fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String {
///         let id_attr = id.map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
///         let class_attr = if classes.is_empty() {
///             String::new()
///         } else {
///             format!(" class=\"{}\"", classes.join(" "))
///         };
///         format!("<h{level}{id_attr}{class_attr}><span class=\"icon\">§</span>")
///     }
///
///     fn render_end(&self, level: u8) -> String {
///         format!("</h{level}>")
///     }
/// }
///
/// let options = MarkdownOptions {
///     components: MarkdownComponents::new().heading(MyCustomHeading),
/// };
/// let html = render_markdown(markdown, Some(&options));
/// ```
pub fn render_markdown(
    content: &str,
    options: Option<&MarkdownOptions>,
    path: Option<&Path>,
    mut route_ctx: Option<&mut PageContext>,
) -> String {
    let content = if let Some(shortcodes) = options.map(|o| &o.shortcodes)
        && !shortcodes.is_empty()
    {
        preprocess_shortcodes(
            content,
            shortcodes,
            route_ctx.as_deref_mut(),
            path.and_then(|p| p.to_str()),
        )
        .unwrap_or_else(|e| panic!("Failed to preprocess shortcodes for {:?}: {}", path, e))
    } else {
        content.to_string()
    };

    let mut slugger = slugger::Slugger::new();
    let mut html_output = String::new();
    let parser_options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_TABLES
        | Options::ENABLE_GFM
        | Options::ENABLE_MATH
        | Options::ENABLE_FOOTNOTES;

    let mut code_block = None;
    let mut code_block_content = String::new();
    let mut in_frontmatter = false;
    let mut in_image = false;
    let mut events = Parser::new_ext(&content, parser_options).collect::<Vec<Event>>();

    let options_with_components = options
        .as_ref()
        .filter(|o| o.components.has_any_components());

    for i in 0..events.len() {
        match &events[i] {
            Event::Start(Tag::MetadataBlock(_)) => {
                in_frontmatter = true;
                continue;
            }
            Event::End(TagEnd::MetadataBlock(_)) => {
                in_frontmatter = false;
                continue;
            }

            // TODO: Write an integration test for assets resolution - erika, 2025-08-27
            Event::Start(Tag::Image {
                dest_url,
                link_type,
                id,
                title,
            }) => {
                // TODO: Figure out a cleaner way to do this, it's a lot of if-lets and checks - erika, 2025-08-27
                let new_event = if dest_url.starts_with("./") || dest_url.starts_with("../") {
                    path.and_then(|p| p.parent())
                        .and_then(|parent| {
                            let resolved = parent.join(dest_url.to_string());
                            route_ctx
                                .as_mut()
                                .and_then(|ctx| ctx.assets.add_image(resolved).url().cloned())
                        })
                        .map(|image_url| {
                            Event::Start(Tag::Image {
                                dest_url: image_url.into(),
                                title: title.clone(),
                                link_type: *link_type,
                                id: id.clone(),
                            })
                        })
                } else {
                    None
                };

                if let Some(event) = new_event {
                    events[i] = event;
                }
            }

            // TODO: Handle this differently so it's compatible with the component system - erika, 2025-08-24
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(fence))) => {
                let (block, begin) = CodeBlock::new(fence);
                code_block = Some(block);
                events[i] = Event::Html(begin.into());
            }

            Event::End(TagEnd::CodeBlock) => {
                if let Some(ref mut code_block) = code_block {
                    let html = code_block.highlight(&code_block_content);
                    events[i] =
                        Event::Html(format!("{}{}", html.unwrap(), "</code></pre>\n").into());
                }
                code_block = None;
                code_block_content.clear();
            }

            // TODO: User should be able to replace the text component too perhaps, but it'd require merging the text events
            Event::Text(text) => {
                if !in_frontmatter {
                    if in_image {
                        // This seem to work to create "an empty event", but it's not ideal. Using `events.remove` is probably
                        // more idiomatic, but it's also less efficient. Wonky situation.
                        events[i] = Event::Html("".into());
                    } else if code_block.is_some() {
                        code_block_content.push_str(text);

                        events[i] = Event::Html("".into());
                    }
                } else {
                    events[i] = Event::Html("".into());
                }
            }

            // Headings
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                // Extract heading content for slug generation only
                let heading_content = if let Some(end_index) = find_matching_heading_end(&events, i)
                {
                    get_text_from_events(&events[i + 1..end_index])
                } else {
                    String::new()
                };

                let slug = slugger.slugify(&heading_content);
                let heading_id = id.as_ref().map(|s| s.as_ref()).unwrap_or(&slug);

                if let Some(component) = options.and_then(|opts| opts.components.heading.as_ref()) {
                    let classes_vec: Vec<&str> = classes.iter().map(|c| c.as_ref()).collect();
                    let custom_html =
                        component.render_start(*level as u8, Some(heading_id), &classes_vec);
                    events[i] = Event::Html(custom_html.into());
                } else {
                    events[i] = Event::Html(
                        format!(
                            "<{} id=\"{}\" class=\"{}\">",
                            level,
                            heading_id,
                            classes.join(" ")
                        )
                        .into(),
                    );
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some(component) = options.and_then(|opts| opts.components.heading.as_ref()) {
                    let custom_html = component.render_end(*level as u8);
                    events[i] = Event::Html(custom_html.into());
                } else {
                    events[i] = Event::Html(format!("</h{}>", *level as u32).into());
                }
            }

            // All other events pass through unchanged
            _ => {}
        }

        // Handle using components for all the different events
        if let Some(options) = options_with_components {
            match &events[i] {
                // Paragraphs
                Event::Start(Tag::Paragraph) => {
                    if let Some(component) = &options.components.paragraph {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Paragraph) => {
                    if let Some(component) = &options.components.paragraph {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Links, i.e [link text](url)
                // TODO: Verify that everything works when using different types of link
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    ..
                }) => {
                    if let Some(component) = &options.components.link {
                        let link_type_converted: LinkType = link_type.into();
                        let title_str = if title.is_empty() {
                            None
                        } else {
                            Some(title.as_ref())
                        };
                        let custom_html = component.render_start(
                            dest_url.as_ref(),
                            title_str,
                            link_type_converted,
                        );
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Link) => {
                    if let Some(component) = &options.components.link {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Images, i.e ![alt text](url)
                // TODO: Verify that everything works when using different types of images
                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    in_image = true;
                    if let Some(component) = &options.components.image {
                        // For images, we need to get the alt text from content between start and end
                        let alt_text = if let Some(end_index) = find_matching_image_end(&events, i)
                        {
                            get_text_from_events(&events[i + 1..end_index])
                        } else {
                            String::new()
                        };
                        let title_str = if title.is_empty() {
                            None
                        } else {
                            Some(title.as_ref())
                        };
                        let custom_html = component.render(dest_url.as_ref(), &alt_text, title_str);
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                Event::End(TagEnd::Image) => {
                    // Images are a bit weird, the alt text is part of the text between the start and end tags
                    // despite in some syntax being actually part of the image tag itself. Perhaps something I'm just not
                    // familiar with.
                    in_image = false;
                }

                // Bold (strong)
                Event::Start(Tag::Strong) => {
                    if let Some(component) = &options.components.strong {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Strong) => {
                    if let Some(component) = &options.components.strong {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Italic (emphasis)
                Event::Start(Tag::Emphasis) => {
                    if let Some(component) = &options.components.emphasis {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Emphasis) => {
                    if let Some(component) = &options.components.emphasis {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Inline Code
                Event::Code(code) => {
                    if let Some(component) = &options.components.code {
                        let custom_html = component.render(code.as_ref());
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Blockquotes
                Event::Start(Tag::BlockQuote(kind)) => {
                    if let Some(component) = &options.components.blockquote {
                        let kind_converted = kind.as_ref().map(|k| k.into());
                        let custom_html = component.render_start(kind_converted);
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::BlockQuote(kind)) => {
                    if let Some(component) = &options.components.blockquote {
                        let kind_converted = kind.as_ref().map(|k| k.into());
                        let custom_html = component.render_end(kind_converted);
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Hard Breaks
                Event::HardBreak => {
                    if let Some(component) = &options.components.hard_break {
                        let custom_html = component.render();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Horizontal Rules
                Event::Rule => {
                    if let Some(component) = &options.components.horizontal_rule {
                        let custom_html = component.render();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // Lists
                Event::Start(Tag::List(first_number)) => {
                    if let Some(component) = &options.components.list {
                        let list_type = if first_number.is_some() {
                            ListType::Ordered
                        } else {
                            ListType::Unordered
                        };
                        let custom_html = component.render_start(list_type, *first_number);
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::List(ordered)) => {
                    if let Some(component) = &options.components.list {
                        let list_type = if *ordered {
                            ListType::Ordered
                        } else {
                            ListType::Unordered
                        };
                        let custom_html = component.render_end(list_type);
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // List Items, i.e. individual - item
                Event::Start(Tag::Item) => {
                    if let Some(component) = &options.components.list_item {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Item) => {
                    if let Some(component) = &options.components.list_item {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // (GFM) Strikethrough, i.e. ~~strikethrough~~
                Event::Start(Tag::Strikethrough) => {
                    if let Some(component) = &options.components.strikethrough {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Strikethrough) => {
                    if let Some(component) = &options.components.strikethrough {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // (GFM) Task List Markers, i.e. - [ ] item
                Event::TaskListMarker(checked) => {
                    if let Some(component) = &options.components.task_list_marker {
                        let custom_html = component.render(*checked);
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // (GFM) Tables, i.e. | Header | Header |
                //                    |--------|--------|
                //                    | Cell   | Cell   |
                //                    |--------|--------|
                Event::Start(Tag::Table(alignments)) => {
                    if let Some(component) = &options.components.table {
                        let alignment_vec: Vec<TableAlignment> = alignments
                            .iter()
                            .map(|a| match a {
                                pulldown_cmark::Alignment::Left => TableAlignment::Left,
                                pulldown_cmark::Alignment::Center => TableAlignment::Center,
                                pulldown_cmark::Alignment::Right => TableAlignment::Right,
                                pulldown_cmark::Alignment::None => TableAlignment::Left,
                            })
                            .collect();
                        let custom_html = component.render_start(&alignment_vec);
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::Table) => {
                    if let Some(component) = &options.components.table {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // (GFM) Table Heads, i.e. | Header | Header |
                Event::Start(Tag::TableHead) => {
                    if let Some(component) = &options.components.table_head {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::TableHead) => {
                    if let Some(component) = &options.components.table_head {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // (GFM) Table Rows, i.e. | Cell | Cell |
                Event::Start(Tag::TableRow) => {
                    if let Some(component) = &options.components.table_row {
                        let custom_html = component.render_start();
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::TableRow) => {
                    if let Some(component) = &options.components.table_row {
                        let custom_html = component.render_end();
                        events[i] = Event::Html(custom_html.into());
                    }
                }

                // (GFM) Table Cells, i.e. individual | Cell |
                Event::Start(Tag::TableCell) => {
                    if let Some(component) = &options.components.table_cell {
                        // For now, assume it's not a header and no specific alignment
                        // TODO: Track context to determine if we're in a table head and column alignment
                        let custom_html = component.render_start(false, None);
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                Event::End(TagEnd::TableCell) => {
                    if let Some(component) = &options.components.table_cell {
                        // TODO: Track context to determine if we're in a table head
                        let custom_html = component.render_end(false);
                        events[i] = Event::Html(custom_html.into());
                    }
                }
                _ => {}
            }
        }
    }

    events.retain(|e| match e {
        Event::Text(content) | Event::Html(content) => !content.is_empty(),
        _ => true,
    });

    push_html(&mut html_output, events.into_iter());
    html_output
}

/// Parse Markdown content with frontmatter and extract headings.
///
/// This function extracts YAML frontmatter from markdown content, deserializes it into the specified type,
/// and automatically populates the headings for table of contents generation.
///
/// ## Example
/// ```rs
/// use maudit::content::{parse_markdown_with_frontmatter, markdown_entry};
///
/// #[markdown_entry]
/// pub struct ArticleContent {
///     pub title: String,
///     pub description: String,
/// }
///
/// let markdown = r#"---
/// title: "My Article"
/// description: "A great article"
/// ---
///
/// # Introduction
///
/// This is the content.
/// "#;
///
/// let parsed: ArticleContent = parse_markdown_with_frontmatter(markdown);
/// assert_eq!(parsed.title, "My Article");
/// assert_eq!(parsed.get_headings().len(), 1);
/// ```
pub fn parse_markdown_with_frontmatter<T>(content: &str) -> T
where
    T: DeserializeOwned + MarkdownContent + InternalMarkdownContent,
{
    let mut slugger = slugger::Slugger::new();

    let mut options = Options::empty();
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS | Options::ENABLE_HEADING_ATTRIBUTES);

    let mut frontmatter = String::new();
    let mut in_frontmatter = false;

    let mut content_events = Vec::new();
    for (event, _) in Parser::new_ext(content, options).into_offset_iter() {
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
    // TODO: Support TOML frontmatters
    let mut parsed = serde_yaml::from_str::<T>(&frontmatter)
        .unwrap_or_else(|e| panic!("Failed to parse YAML frontmatter: {}, {}", e, frontmatter));

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
    parsed
}

fn find_matching_heading_end(events: &[Event], start_index: usize) -> Option<usize> {
    events[start_index + 1..]
        .iter()
        .position(|event| matches!(event, Event::End(TagEnd::Heading(_))))
        .map(|offset| start_index + 1 + offset)
}

fn find_matching_image_end(events: &[Event], start_index: usize) -> Option<usize> {
    events[start_index + 1..]
        .iter()
        .position(|event| matches!(event, Event::End(TagEnd::Image)))
        .map(|offset| start_index + 1 + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown_rendering() {
        let markdown = r#"# Hello, world!

This is a **bold** text.

## Subheading

More content here."#;

        let html = render_markdown(markdown, None, None, None);

        // Test basic markdown rendering
        assert!(html.contains("<h1"));
        assert!(html.contains("<h2"));
        assert!(html.contains("</h1>"));
        assert!(html.contains("</h2>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("Hello, world!"));
        assert!(html.contains("Subheading"));
    }

    #[test]
    fn test_rendering_with_empty_components() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new(),
            ..Default::default()
        };
        let markdown = r#"# Hello, world!"#;

        let html = render_markdown(markdown, Some(&options), None, None);
        let default_html = render_markdown(markdown, None, None, None);

        // Should be the same as default rendering when no custom components are provided
        assert_eq!(html, default_html);
    }

    #[test]
    fn test_default_heading_behavior_with_and_without_options() {
        let markdown = r#"# Main Title

## Subheading

### Another Level"#;

        // Render without any options
        let html_no_options = render_markdown(markdown, None, None, None);

        // Render with options but no custom heading component
        let options_no_heading = MarkdownOptions {
            components: MarkdownComponents::new(),
            ..Default::default()
        };
        let html_with_empty_options =
            render_markdown(markdown, Some(&options_no_heading), None, None);

        // Both should produce identical output
        assert_eq!(html_no_options, html_with_empty_options);

        // Both should have default heading behavior (id attributes and proper HTML structure)
        assert!(html_no_options.contains("id=\""));
        assert!(html_no_options.contains("<h1"));
        assert!(html_no_options.contains("<h2"));
        assert!(html_no_options.contains("<h3"));
        assert!(html_with_empty_options.contains("id=\""));
    }

    // Helper function to create test shortcodes
    fn create_test_shortcodes() -> MarkdownShortcodes {
        let mut shortcodes = MarkdownShortcodes::new();

        shortcodes.register("simple", |_args, _| "SIMPLE_OUTPUT".to_string());

        shortcodes.register("greet", |args, _| {
            let name = args.get_str("name").unwrap_or("World");
            format!("Hello, {}!", name)
        });

        shortcodes.register("date", |args, _| {
            let format = args.get_str("format").unwrap_or("default");
            format!("DATE[{}]", format)
        });

        shortcodes.register("highlight", |args, _| {
            let lang = args.get_str("lang").unwrap_or("text");
            let body = args.get_str("body").unwrap_or("");
            format!("<code class=\"lang-{}\">{}</code>", lang, body)
        });

        shortcodes.register("alert", |args, _| {
            let alert_type = args.get_str("type").unwrap_or("info");
            let body = args.get_str("body").unwrap_or("");
            format!("<div class=\"alert alert-{}\">{}</div>", alert_type, body)
        });

        shortcodes.register("section", |args, _| {
            let title = args.get_str("title").unwrap_or("");
            let body = args.get_str("body").unwrap_or("");
            if title.is_empty() {
                format!("<section>{}</section>", body)
            } else {
                format!("<section data-title=\"{}\">{}</section>", title, body)
            }
        });

        shortcodes
    }

    #[test]
    fn test_markdown_with_shortcodes_basic() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = "# {{ greet name=Title /}}\n\nHello {{ simple /}}!";
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<h1"));
        assert!(html.contains("Hello, Title!"));
        assert!(html.contains("Hello SIMPLE_OUTPUT!"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_headings() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"# {{ greet name=Main /}}

## Section {{ date format=short /}}

### {{ simple /}} Chapter"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<h1"));
        assert!(html.contains("Hello, Main!"));
        assert!(html.contains("<h2"));
        assert!(html.contains("Section DATE[short]"));
        assert!(html.contains("<h3"));
        assert!(html.contains("SIMPLE_OUTPUT Chapter"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_emphasis() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = "*{{ greet name=Italic /}}* and **{{ simple /}}**";
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<em>Hello, Italic!</em>"));
        assert!(html.contains("<strong>SIMPLE_OUTPUT</strong>"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_lists() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"1. {{ greet name=First /}}
2. {{ simple /}}
3. {{ date format=iso /}}"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<ol>"));
        assert!(html.contains("<li>Hello, First!</li>"));
        assert!(html.contains("<li>SIMPLE_OUTPUT</li>"));
        assert!(html.contains("<li>DATE[iso]</li>"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_tables() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"| Name | Greeting |
|------|----------|
| Alice | {{ greet name=Alice /}} |
| Bob | {{ simple /}} |"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<th>Greeting</th>"));
        assert!(html.contains("<td>Alice</td>"));
        assert!(html.contains("<td>Hello, Alice!</td>"));
        assert!(html.contains("<td>Bob</td>"));
        assert!(html.contains("<td>SIMPLE_OUTPUT</td>"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_blockquotes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"> {{ greet name=Quote /}}
>
> {{ simple /}}"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Hello, Quote!"));
        assert!(html.contains("SIMPLE_OUTPUT"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_code_blocks() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"```rust
fn main() {
    println!("{{ greet name=Rust /}}");
    // {{ simple /}}
}
```"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<pre"));
        assert!(html.contains("<code"));
        assert!(html.contains("Hello, Rust!"));
        assert!(html.contains("SIMPLE_OUTPUT"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_links() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"[{{ greet name=Link /}}](https://example.com "{{ simple /}}")

![{{ greet name=Alt /}}](image.jpg "{{ date format=title /}}")"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<a href=\"https://example.com\""));
        assert!(html.contains("title=\"SIMPLE_OUTPUT\""));
        assert!(html.contains(">Hello, Link!</a>"));
        assert!(html.contains("<img src=\"image.jpg\""));
        assert!(html.contains("alt=\"Hello, Alt!\""));
        assert!(html.contains("title=\"DATE[title]\""));
    }

    #[test]
    fn test_markdown_with_block_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"{{ highlight lang=rust }}
fn main() {
    println!("{{ greet name=World /}}");
}
{{ /highlight }}"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<code class=\"lang-rust\">"));
        assert!(html.contains("Hello, World!"));
        assert!(html.contains("</code>"));
    }

    #[test]
    fn test_markdown_with_nested_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"{{ alert type=warning }}
## {{ greet name=Alert /}}

{{ simple /}} content here.
{{ /alert }}"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<div class=\"alert alert-warning\">"));
        // The markdown inside the shortcode becomes raw text, not processed markdown
        assert!(html.contains("## Hello, Alert!"));
        assert!(html.contains("SIMPLE_OUTPUT content"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn test_markdown_with_deeply_nested_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"{{ section title=Main }}
# {{ greet name=Header /}}

{{ alert type=info }}
**{{ greet name=Bold /}}** and *{{ simple /}}*
{{ /alert }}
{{ /section }}"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<section data-title=\"Main\">"));
        // The markdown inside shortcodes becomes raw text, not processed
        assert!(html.contains("# Hello, Header!"));
        assert!(html.contains("<div class=\"alert alert-info\">"));
        assert!(html.contains("**Hello, Bold!** and *SIMPLE_OUTPUT*"));
        assert!(html.contains("</div>"));
        assert!(html.contains("</section>"));
    }

    #[test]
    fn test_markdown_with_shortcodes_in_frontmatter() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"---
title: {{ greet name=Blog /}}
date: {{ date format=iso /}}
tags: [{{ simple /}}, {{ greet name=Tutorial /}}]
---

# {{ greet name=Content /}}

Welcome to {{ simple /}}!"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        // The HTML shouldn't contain the frontmatter, but shortcodes in content should be processed
        assert!(!html.contains("---"));
        assert!(!html.contains("title:"));
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello, Content!"));
        assert!(html.contains("Welcome to SIMPLE_OUTPUT!"));
    }

    #[test]
    fn test_markdown_with_task_lists_and_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"- [x] {{ greet name=Done /}}
- [ ] {{ simple /}}
- [ ] {{ date format=todo /}}"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<ul>"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked=\"\""));
        assert!(html.contains("Hello, Done!"));
        assert!(html.contains("SIMPLE_OUTPUT"));
        assert!(html.contains("DATE[todo]"));
    }

    #[test]
    fn test_markdown_with_strikethrough_and_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = "~~{{ greet name=Deleted /}}~~ and {{ simple /}}";
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("<del>Hello, Deleted!</del>"));
        assert!(html.contains("and SIMPLE_OUTPUT"));
    }

    #[test]
    fn test_markdown_real_world_blog_post_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"---
title: {{ greet name=BlogPost /}}
date: {{ date format=iso /}}
---

# {{ greet name=Reader /}}!

Welcome to my blog about **{{ simple /}}**.

## What we'll cover

1. {{ greet name=Introduction /}}
2. {{ simple /}} basics
3. Advanced {{ greet name=Techniques /}}

{{ alert type=info }}
💡 **Tip**: Remember {{ greet name=This /}}!
{{ /alert }}

### Code Example

{{ highlight lang=rust }}
fn main() {
    println!("{{ greet name=World /}}!");
}
{{ /highlight }}

### Task List

- [x] {{ greet name=Setup /}}
- [ ] {{ simple /}}
- [ ] {{ greet name=Deploy /}}

> "{{ greet name=Quote /}}" - *{{ simple /}}*

Check out [this link](https://example.com "{{ greet name=Title /}}")!"#;

        let html = render_markdown(markdown, Some(&options), None, None);

        // Test various HTML elements are properly rendered with shortcodes
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello, Reader!"));
        assert!(html.contains("<strong>SIMPLE_OUTPUT</strong>"));
        assert!(html.contains("<h2"));
        assert!(html.contains("<ol>"));
        assert!(html.contains("Hello, Introduction!"));
        assert!(html.contains("<div class=\"alert alert-info\">"));
        assert!(html.contains("Remember Hello, This!"));
        assert!(html.contains("<code class=\"lang-rust\">"));
        assert!(html.contains("Hello, World!"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("Hello, Setup!"));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Hello, Quote!"));
        assert!(html.contains("<a href=\"https://example.com\""));
        assert!(html.contains("title=\"Hello, Title!\""));
    }

    #[test]
    fn test_markdown_with_math_and_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let options = MarkdownOptions {
            shortcodes,
            ..Default::default()
        };

        let markdown = r#"Inline math with {{ simple /}}: $x = {{ greet name=Variable /}}$

Block math:
$$
{{ greet name=Equation /}}
$$"#;
        let html = render_markdown(markdown, Some(&options), None, None);

        assert!(html.contains("SIMPLE_OUTPUT"));
        // Math expressions might be processed differently
        assert!(html.contains("Hello, Variable!"));
        assert!(html.contains("Hello, Equation!"));
    }
}
