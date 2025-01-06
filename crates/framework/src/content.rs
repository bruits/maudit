use std::any::Any;

use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::de::DeserializeOwned;

use crate::page::RouteParams;

pub struct ContentEntry<T> {
    pub id: String,
    render: Box<dyn Fn(&str) -> String>,
    pub raw_content: String,
    pub data: T,
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

pub struct ContentSource<T = Untyped> {
    pub name: String,
    pub entries: Vec<ContentEntry<T>>,
    pub(crate) init_method: Box<dyn Fn() -> Vec<ContentEntry<T>>>,
}

pub struct Content<'a> {
    sources: &'a Vec<Box<dyn ContentSourceInternal>>,
    accessed_sources: FxHashSet<String>,
}

impl Content<'_> {
    pub fn new(sources: &Vec<Box<dyn ContentSourceInternal>>) -> Content {
        Content {
            sources,
            accessed_sources: FxHashSet::default(),
        }
    }

    pub(crate) fn get_accessed_sources(&self) -> &FxHashSet<String> {
        &self.accessed_sources
    }

    pub fn get_untyped_source(&mut self, name: &str) -> &ContentSource<Untyped> {
        self.sources
            .iter()
            .find_map(
                |source| match source.as_any().downcast_ref::<ContentSource<Untyped>>() {
                    Some(source) if source.name == name => {
                        self.accessed_sources.insert(name.to_string());
                        Some(source)
                    }
                    _ => None,
                },
            )
            .unwrap_or_else(|| panic!("Content source with name '{}' not found", name))
    }

    pub fn get_untyped_source_safe(&mut self, name: &str) -> Option<&ContentSource<Untyped>> {
        self.sources.iter().find_map(|source| {
            match source.as_any().downcast_ref::<ContentSource<Untyped>>() {
                Some(source) if source.name == name => {
                    self.accessed_sources.insert(name.to_string());
                    Some(source)
                }
                _ => None,
            }
        })
    }

    pub fn get_source<T: 'static>(&mut self, name: &str) -> &ContentSource<T> {
        self.sources
            .iter()
            .find_map(
                |source| match source.as_any().downcast_ref::<ContentSource<T>>() {
                    Some(source) if source.name == name => {
                        self.accessed_sources.insert(name.to_string());
                        Some(source)
                    }
                    _ => None,
                },
            )
            .unwrap_or_else(|| panic!("Content source with name '{}' not found", name))
    }

    pub fn get_source_safe<T: 'static>(&mut self, name: &str) -> Option<&ContentSource<T>> {
        self.sources.iter().find_map(|source| {
            match source.as_any().downcast_ref::<ContentSource<T>>() {
                Some(source) if source.name == name => {
                    self.accessed_sources.insert(name.to_string());
                    Some(source)
                }
                _ => None,
            }
        })
    }
}

impl<T> ContentSource<T> {
    pub fn new<P>(name: P, entries: Box<dyn Fn() -> Vec<ContentEntry<T>>>) -> Self
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

pub trait ContentSourceInternal {
    fn init(&mut self);
    fn get_name(&self) -> &str;
    fn as_any(&self) -> &dyn Any; // Used for type checking at runtime
}

impl<T: 'static> ContentSourceInternal for ContentSource<T> {
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

pub fn glob_markdown<T>(pattern: &str) -> Vec<ContentEntry<T>>
where
    T: DeserializeOwned,
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
            Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_TABLES
                | Options::ENABLE_FOOTNOTES
                | Options::ENABLE_TASKLISTS
                | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,
        );
        let mut frontmatter = String::new();

        let mut in_frontmatter = false;
        for (event, _) in Parser::new_ext(&content, options).into_offset_iter() {
            match event {
                Event::Start(Tag::MetadataBlock(_)) => {
                    in_frontmatter = true;
                }
                Event::End(TagEnd::MetadataBlock(_)) => {
                    in_frontmatter = false;
                }
                Event::Text(ref text) => {
                    if in_frontmatter {
                        frontmatter.push_str(text);
                    }
                }
                _ => {}
            }
        }

        let parsed = serde_yml::from_str::<T>(&frontmatter).unwrap();

        entries.push(ContentEntry {
            id,
            render: Box::new(render_markdown),
            raw_content: content,
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
