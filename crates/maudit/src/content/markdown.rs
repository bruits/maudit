use std::sync::Arc;

use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::de::DeserializeOwned;

pub mod components;

use components::{LinkType, ListType, MarkdownComponents, TableAlignment};

use super::{highlight::CodeBlock, slugger, ContentEntry};

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
///   fn render(&self, ctx: &mut RouteContext) -> Markup {
///     let articles = ctx.content.get_source::<ArticleContent>("articles");
///     let article = articles.get_entry("my-article");
///     let headings = article.data().get_headings(); // returns a Vec<MarkdownHeading>
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
///         h1 { (article.data().title) }
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
}

impl MarkdownOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_components(components: MarkdownComponents) -> Self {
        Self { components }
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

        if let Some(extension) = entry.extension() {
            if extension != "md" {
                warn!("Other file types than Markdown are not supported yet");
                continue;
            }
        }

        let id = entry.file_stem().unwrap().to_str().unwrap().to_string();
        let content = std::fs::read_to_string(&entry).unwrap();

        // Clone content for the closure
        let content_clone = content.clone();
        let data_loader = Box::new(move || {
            let mut slugger = slugger::Slugger::new();

            let mut options = Options::empty();
            options.insert(
                Options::ENABLE_YAML_STYLE_METADATA_BLOCKS | Options::ENABLE_HEADING_ATTRIBUTES,
            );

            let mut frontmatter = String::new();
            let mut in_frontmatter = false;

            let mut content_events = Vec::new();
            for (event, _) in Parser::new_ext(&content_clone, options).into_offset_iter() {
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
                let heading_content =
                    get_text_from_events(&content_events[heading.start..heading.end]);
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
        });

        // Perhaps not ideal, but I don't know better. We're at the "get it working" stage - erika, 2025-08-24
        // Ideally, we'd at least avoid the allocation here whenever `options` is None, not sure how to do that ergonomically
        let opts = options.clone();

        entries.push(ContentEntry::new_lazy(
            id,
            Some(Box::new(move |content: &str| {
                render_markdown(content, opts.as_deref())
            })),
            Some(content),
            data_loader,
            Some(entry),
        ));
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
pub fn render_markdown(content: &str, options: Option<&MarkdownOptions>) -> String {
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
    let mut events = Vec::new();

    // Do a first pass to collect body events
    for event in Parser::new_ext(content, parser_options) {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                in_frontmatter = true;
            }
            Event::End(TagEnd::MetadataBlock(_)) => {
                in_frontmatter = false;
            }
            Event::Text(ref text) => {
                if !in_frontmatter {
                    if code_block.is_some() {
                        code_block_content.push_str(text);
                    } else {
                        events.push(event);
                    }
                }
            }

            // TODO: Handle this differently so it's compatible with the component system - erika, 2025-08-24
            Event::Start(Tag::CodeBlock(ref kind)) => {
                if let CodeBlockKind::Fenced(ref fence) = kind {
                    let (block, begin) = CodeBlock::new(fence);
                    code_block = Some(block);
                    events.push(Event::Html(begin.into()));
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(ref mut code_block) = code_block {
                    let html = code_block.highlight(&code_block_content);
                    events.push(Event::Html(html.unwrap().into()));
                }
                code_block = None;
                code_block_content.clear();
                events.push(Event::Html("</code></pre>\n".into()));
            }

            _ => {
                events.push(event);
            }
        }
    }

    // If we don't have a custom heading component, use default heading rendering
    if options.is_none_or(|o| o.components.heading.is_none()) {
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
    }

    // Second pass: transform events with custom components only if needed
    let final_events = match options {
        Some(options) if options.components.has_any_components() => {
            transform_events_with_components(&events, &options.components, &mut slugger)
        }
        _ => {
            // No options, no components, or empty components - use events as-is
            events
        }
    };

    pulldown_cmark::html::push_html(&mut html_output, final_events.into_iter());
    html_output
}

fn transform_events_with_components<'a>(
    events: &'a [Event],
    components: &MarkdownComponents,
    slugger: &mut slugger::Slugger,
) -> Vec<Event<'a>> {
    let mut transformed = Vec::new();
    let mut i = 0;

    while i < events.len() {
        let event = &events[i];

        match event {
            // Headings
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                if let Some(component) = &components.heading {
                    let heading_content =
                        if let Some(end_index) = find_matching_heading_end(events, i) {
                            get_text_from_events(&events[i + 1..end_index])
                        } else {
                            String::new()
                        };
                    let slug = slugger.slugify(&heading_content);
                    let heading_id = id.as_ref().map(|s| s.as_ref()).unwrap_or(&slug);
                    let classes_vec: Vec<&str> = classes.iter().map(|c| c.as_ref()).collect();

                    let custom_html =
                        component.render_start(*level as u8, Some(heading_id), &classes_vec);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some(component) = &components.heading {
                    let custom_html = component.render_end(*level as u8);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Paragraphs
            Event::Start(Tag::Paragraph) => {
                if let Some(component) = &components.paragraph {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(component) = &components.paragraph {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Links
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                if let Some(component) = &components.link {
                    let link_type_converted: LinkType = link_type.into();
                    let title_str = if title.is_empty() {
                        None
                    } else {
                        Some(title.as_ref())
                    };
                    let custom_html =
                        component.render_start(dest_url.as_ref(), title_str, link_type_converted);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some(component) = &components.link {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Images
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                if let Some(component) = &components.image {
                    // For images, we need to get the alt text from content between start and end
                    let alt_text = if let Some(end_index) = find_matching_image_end(events, i) {
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
                    transformed.push(Event::Html(custom_html.into()));
                    // Skip to the end tag
                    if let Some(end_index) = find_matching_image_end(events, i) {
                        i = end_index;
                    }
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Image) => {
                // Only add this if we didn't handle it above with custom component
                if components.image.is_none() {
                    transformed.push(event.clone());
                }
            }

            // Bold (strong)
            Event::Start(Tag::Strong) => {
                if let Some(component) = &components.strong {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Strong) => {
                if let Some(component) = &components.strong {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Italic (emphasis)
            Event::Start(Tag::Emphasis) => {
                if let Some(component) = &components.emphasis {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Emphasis) => {
                if let Some(component) = &components.emphasis {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Inline Code, i.e `something`
            Event::Code(code) => {
                if let Some(component) = &components.code {
                    let custom_html = component.render(code.as_ref());
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Blockquotes, i.e. > quote
            Event::Start(Tag::BlockQuote(kind)) => {
                if let Some(component) = &components.blockquote {
                    let kind_converted = kind.as_ref().map(|k| k.into());
                    let custom_html = component.render_start(kind_converted);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::BlockQuote(kind)) => {
                if let Some(component) = &components.blockquote {
                    let kind_converted = kind.as_ref().map(|k| k.into());
                    let custom_html = component.render_end(kind_converted);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Hard Breaks, i.e. double spaces at the end of a line
            Event::HardBreak => {
                if let Some(component) = &components.hard_break {
                    let custom_html = component.render();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Horizontal Rules, i.e. --- -> <hr />
            Event::Rule => {
                if let Some(component) = &components.horizontal_rule {
                    let custom_html = component.render();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // Lists, i.e. - item
            Event::Start(Tag::List(first_number)) => {
                if let Some(component) = &components.list {
                    let list_type = if first_number.is_some() {
                        ListType::Ordered
                    } else {
                        ListType::Unordered
                    };
                    let custom_html = component.render_start(list_type, *first_number);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::List(ordered)) => {
                if let Some(component) = &components.list {
                    let list_type = if *ordered {
                        ListType::Ordered
                    } else {
                        ListType::Unordered
                    };
                    let custom_html = component.render_end(list_type);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // List Items, i.e. individual - item
            Event::Start(Tag::Item) => {
                if let Some(component) = &components.list_item {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Item) => {
                if let Some(component) = &components.list_item {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // (GFM) Strikethrough, i.e. ~~strikethrough~~
            Event::Start(Tag::Strikethrough) => {
                if let Some(component) = &components.strikethrough {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Strikethrough) => {
                if let Some(component) = &components.strikethrough {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // (GFM) Task List Markers, i.e. - [ ] item
            Event::TaskListMarker(checked) => {
                if let Some(component) = &components.task_list_marker {
                    let custom_html = component.render(*checked);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // (GFM) Tables, i.e. | Header | Header |
            //                    |--------|--------|
            //                    | Cell   | Cell   |
            //                    |--------|--------|
            Event::Start(Tag::Table(alignments)) => {
                if let Some(component) = &components.table {
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
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(component) = &components.table {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // (GFM) Table Heads, i.e. | Header | Header |
            Event::Start(Tag::TableHead) => {
                if let Some(component) = &components.table_head {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(component) = &components.table_head {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // (GFM) Table Rows, i.e. | Cell | Cell |
            Event::Start(Tag::TableRow) => {
                if let Some(component) = &components.table_row {
                    let custom_html = component.render_start();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(component) = &components.table_row {
                    let custom_html = component.render_end();
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // (GFM) Table Cells, i.e. individual | Cell |
            Event::Start(Tag::TableCell) => {
                if let Some(component) = &components.table_cell {
                    // For now, assume it's not a header and no specific alignment
                    // TODO: Track context to determine if we're in a table head and column alignment
                    let custom_html = component.render_start(false, None);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(component) = &components.table_cell {
                    // TODO: Track context to determine if we're in a table head
                    let custom_html = component.render_end(false);
                    transformed.push(Event::Html(custom_html.into()));
                } else {
                    transformed.push(event.clone());
                }
            }

            // All other events pass through unchanged
            _ => {
                transformed.push(event.clone());
            }
        }
        i += 1;
    }

    transformed
}

fn find_matching_heading_end(events: &[Event], start_index: usize) -> Option<usize> {
    for (i, event) in events.iter().enumerate().skip(start_index + 1) {
        if matches!(event, Event::End(TagEnd::Heading(_))) {
            return Some(i);
        }
    }
    None
}

fn find_matching_image_end(events: &[Event], start_index: usize) -> Option<usize> {
    for (i, event) in events.iter().enumerate().skip(start_index + 1) {
        if matches!(event, Event::End(TagEnd::Image)) {
            return Some(i);
        }
    }
    None
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

        let html = render_markdown(markdown, None);

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
        };
        let markdown = r#"# Hello, world!"#;

        let html = render_markdown(markdown, Some(&options));
        let default_html = render_markdown(markdown, None);

        // Should be the same as default rendering when no custom components are provided
        assert_eq!(html, default_html);
    }

    #[test]
    fn test_default_heading_behavior_with_and_without_options() {
        let markdown = r#"# Main Title

## Subheading

### Another Level"#;

        // Render without any options
        let html_no_options = render_markdown(markdown, None);

        // Render with options but no custom heading component
        let options_no_heading = MarkdownOptions {
            components: MarkdownComponents::new(),
        };
        let html_with_empty_options = render_markdown(markdown, Some(&options_no_heading));

        // Both should produce identical output
        assert_eq!(html_no_options, html_with_empty_options);
        
        // Both should have default heading behavior (id attributes and proper HTML structure)
        assert!(html_no_options.contains("id=\""));
        assert!(html_no_options.contains("<h1"));
        assert!(html_no_options.contains("<h2"));
        assert!(html_no_options.contains("<h3"));
        assert!(html_with_empty_options.contains("id=\""));
    }


}
