use std::{any::Any, path::PathBuf};

use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;

use crate::page::RouteParams;
pub use maudit_macros::markdown_entry;

pub struct Content<'a> {
    sources: &'a Vec<Box<dyn ContentSourceInternal>>,
}

impl Content<'_> {
    pub fn new(sources: &Vec<Box<dyn ContentSourceInternal>>) -> Content {
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

pub type Untyped = FxHashMap<String, String>;

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

#[derive(Debug, Clone)]
pub struct MarkdownHeading {
    pub title: String,
    pub id: String,
    pub level: u8,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, Option<String>)>,
}

pub trait MarkdownContent {
    fn get_headings(&self) -> &Vec<MarkdownHeading>;
}

pub trait InternalMarkdownContent {
    fn set_headings(&mut self, headings: Vec<MarkdownHeading>);
}

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

pub fn glob_markdown<T>(pattern: &str) -> Vec<ContentEntry<T>>
where
    T: DeserializeOwned + MarkdownContent + InternalMarkdownContent,
{
    let mut entries = vec![];

    for entry in glob_fs(pattern).unwrap() {
        let entry = entry.unwrap();
        let id = entry.file_stem().unwrap().to_str().unwrap().to_string();
        let content = std::fs::read_to_string(&entry).unwrap();

        let extension = match entry.extension() {
            Some(extension) => extension,
            None => continue,
        };

        if extension != "md" {
            warn!("Other file types than Markdown are not supported yet");
            continue;
        }

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
