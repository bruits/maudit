use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::de::DeserializeOwned;

use super::components::MarkdownComponents;

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
///       "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
///     ],
///     BuildOptions::default(),
///   )
/// }
/// ```
pub fn glob_markdown<T>(pattern: &str) -> Vec<ContentEntry<T>>
where
    T: DeserializeOwned + MarkdownContent + InternalMarkdownContent + Send + Sync + 'static,
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

        entries.push(ContentEntry::new_lazy(
            id,
            Some(Box::new(render_markdown)),
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

/// Render Markdown content to HTML with custom components.
///
/// ## Example
/// ```rs
/// use maudit::content::{render_markdown_with_components, MarkdownComponents};
/// use maudit::content::components::{MarkdownComponent, CustomHeading};
/// use pulldown_cmark::{Event, Tag, TagEnd};
///
/// // Define a custom component
/// struct MyCustomHeading;
///
/// impl MarkdownComponent for MyCustomHeading {
///     fn render_start(&self, event: &Event) -> Option<String> {
///         if let Event::Start(Tag::Heading { level, id, classes, .. }) = event {
///             let level_num = *level as u8;
///             let id_attr = id.as_ref().map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
///             let class_attr = if classes.is_empty() {
///                 String::new()
///             } else {
///                 format!(" class=\"{}\"", classes.iter().map(|c| c.as_ref()).collect::<Vec<_>>().join(" "))
///             };
///             Some(format!("<h{level_num}{id_attr}{class_attr}><span class=\"icon\">§</span>"))
///         } else {
///             None
///         }
///     }
///
///     fn render_end(&self, event: &Event) -> Option<String> {
///         if let Event::End(TagEnd::Heading(level)) = event {
///             let level_num = *level as u8;
///             Some(format!("</h{level_num}>"))
///         } else {
///             None
///         }
///     }
/// }
///
/// let components = MarkdownComponents::new().heading(MyCustomHeading);
/// let markdown = r#"# Hello, world!"#;
/// let html = render_markdown_with_components(markdown, &components);
/// ```
pub fn render_markdown_with_components(content: &str, components: &MarkdownComponents) -> String {
    let mut slugger = slugger::Slugger::new();
    let mut html_output = String::new();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let mut code_block = None;
    let mut code_block_content = String::new();
    let mut in_frontmatter = false;
    let mut events = Vec::new();

    // First pass: collect events, handle frontmatter and code blocks
    for (event, _) in Parser::new_ext(content, options).into_offset_iter() {
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

    // Second pass: transform events with custom components
    let transformed_events = transform_events_with_components(&events, components, &mut slugger);

    pulldown_cmark::html::push_html(&mut html_output, transformed_events.into_iter());
    html_output
}

/// Render Markdown content to HTML.
///
/// ## Example
/// ```rs
/// use maudit::content::render_markdown;
/// let markdown = r#"# Hello, world!"#;
/// let html = render_markdown(markdown);
/// ```
pub fn render_markdown(content: &str) -> String {
    render_markdown_with_components(content, &MarkdownComponents::default())
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

        if let Some(component) = components.find_component(event) {
            match event {
                Event::Start(_) => {
                    // Use custom component for start tag
                    if let Some(custom_html) = component.render_start(event) {
                        transformed.push(Event::Html(custom_html.into()));
                    } else {
                        // Fallback to default behavior
                        match event {
                            Event::Start(Tag::Heading {
                                level, id, classes, ..
                            }) => {
                                let heading_content =
                                    if let Some(end_index) = find_matching_heading_end(events, i) {
                                        get_text_from_events(&events[i + 1..end_index])
                                    } else {
                                        String::new()
                                    };
                                let slug = slugger.slugify(&heading_content);
                                let heading_id = id.as_ref().map(|s| s.to_string()).unwrap_or(slug);
                                let classes_vec: Vec<String> =
                                    classes.iter().map(|c| c.to_string()).collect();

                                transformed.push(Event::Html(
                                    format!(
                                        "<h{} id=\"{}\" class=\"{}\">",
                                        level,
                                        heading_id,
                                        classes_vec.join(" ")
                                    )
                                    .into(),
                                ));
                            }
                            _ => transformed.push(event.clone()),
                        }
                    }
                }
                Event::End(_) => {
                    // Use custom component for end tag
                    if let Some(custom_html) = component.render_end(event) {
                        transformed.push(Event::Html(custom_html.into()));
                    } else {
                        // Keep the original end event
                        transformed.push(event.clone());
                    }
                }
                _ => {
                    transformed.push(event.clone());
                }
            }
        } else {
            // Handle default heading logic for backwards compatibility
            match event {
                Event::Start(Tag::Heading {
                    level, id, classes, ..
                }) => {
                    let heading_content =
                        if let Some(end_index) = find_matching_heading_end(events, i) {
                            get_text_from_events(&events[i + 1..end_index])
                        } else {
                            String::new()
                        };
                    let slug = slugger.slugify(&heading_content);
                    let heading_id = id.as_ref().map(|s| s.to_string()).unwrap_or(slug);
                    let classes_vec: Vec<String> = classes.iter().map(|c| c.to_string()).collect();

                    transformed.push(Event::Html(
                        format!(
                            "<h{} id=\"{}\" class=\"{}\">",
                            level,
                            heading_id,
                            classes_vec.join(" ")
                        )
                        .into(),
                    ));
                }
                _ => {
                    transformed.push(event.clone());
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::components::MarkdownComponent;
    use pulldown_cmark::{Event, Tag, TagEnd};

    // Define a custom heading component for testing
    struct TestCustomHeading;

    impl MarkdownComponent for TestCustomHeading {
        fn render_start(&self, event: &Event) -> Option<String> {
            if let Event::Start(Tag::Heading {
                level, id, classes, ..
            }) = event
            {
                let level_num = *level as u8;
                let id_attr = id
                    .as_ref()
                    .map(|i| format!(" id=\"{}\"", i))
                    .unwrap_or_default();
                let class_attr = if classes.is_empty() {
                    String::new()
                } else {
                    format!(
                        " class=\"{}\"",
                        classes
                            .iter()
                            .map(|c| c.as_ref())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                };
                Some(format!("<h{}{}{}>🎯", level_num, id_attr, class_attr))
            } else {
                None
            }
        }

        fn render_end(&self, event: &Event) -> Option<String> {
            if let Event::End(TagEnd::Heading(level)) = event {
                let level_num = *level as u8;
                Some(format!("</h{}>", level_num))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_custom_heading_component() {
        let components = MarkdownComponents::new().heading(TestCustomHeading);
        let markdown = r#"# Hello, world!

This is a **bold** text.

## Subheading

More content here."#;

        let html = render_markdown_with_components(markdown, &components);

        // Test that custom heading component is used
        assert!(html.contains("🎯"));

        // Test that nested content (bold) is preserved
        assert!(html.contains("<strong>bold</strong>"));

        // Test that multiple heading levels work
        assert!(html.contains("<h1"));
        assert!(html.contains("<h2"));
        assert!(html.contains("</h1>"));
        assert!(html.contains("</h2>"));
    }

    #[test]
    fn test_default_rendering_without_components() {
        let components = MarkdownComponents::new();
        let markdown = r#"# Hello, world!"#;

        let html = render_markdown_with_components(markdown, &components);
        let default_html = render_markdown(markdown);

        // Should be the same as default rendering
        assert_eq!(html, default_html);
    }
}
