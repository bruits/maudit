use std::any::Any;

use glob::glob as glob_fs;
use log::warn;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;

use crate::page::RouteParams;

pub struct ContentEntry<T> {
    pub id: String,
    pub render: Box<dyn Fn() -> String>,
    pub data: T,
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

    pub fn get_untyped_source(&self, name: &str) -> &ContentSource<Untyped> {
        self.0
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
        self.0.iter().find_map(|source| {
            match source.as_any().downcast_ref::<ContentSource<Untyped>>() {
                Some(source) if source.name == name => Some(source),
                _ => None,
            }
        })
    }

    pub fn get_source<T: 'static>(&self, name: &str) -> &ContentSource<T> {
        self.0
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
        self.0.iter().find_map(
            |source| match source.as_any().downcast_ref::<ContentSource<T>>() {
                Some(source) if source.name == name => Some(source),
                _ => None,
            },
        )
    }
}

pub struct ContentSource<T = Untyped> {
    pub name: String,
    pub entries: Vec<ContentEntry<T>>,
    pub(crate) init_method: Box<dyn Fn() -> Vec<ContentEntry<T>>>,
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
        let content_clone = content.clone();

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
        let mut events = Vec::new();
        let mut frontmatter = String::new();

        let mut in_frontmatter = false;
        for (event, _) in Parser::new_ext(&content_clone, options).into_offset_iter() {
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
                    } else {
                        events.push(event);
                    }
                }
                _ => events.push(event),
            }
        }

        let html_output = events.iter().fold(String::new(), |mut acc, event| {
            pulldown_cmark::html::push_html(&mut acc, std::iter::once(event.clone()));
            acc
        });

        let parsed = serde_yml::from_str::<T>(&frontmatter).unwrap();

        entries.push(ContentEntry {
            id,
            render: { Box::new(move || html_output.to_string()) },
            data: parsed,
        });
    }

    entries
}
