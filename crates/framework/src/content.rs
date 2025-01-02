use std::any::Any;

use glob::glob as glob_fs;
use log::warn;
use markdown::{mdast::Node, to_html_with_options, to_mdast, Constructs, Options, ParseOptions};
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
    fn from(collections: Vec<Box<dyn ContentSourceInternal>>) -> Self {
        Self(collections)
    }
}

impl ContentSources {
    pub fn new(collections: Vec<Box<dyn ContentSourceInternal>>) -> Self {
        Self(collections)
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
            .unwrap_or_else(|| panic!("Collection with name '{}' not found", name))
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
            .unwrap_or_else(|| panic!("Collection with name '{}' not found", name))
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
    pub(crate) init_methods: Box<dyn Fn() -> Vec<ContentEntry<T>>>,
}

impl<T> ContentSource<T> {
    pub fn new<P>(name: P, entries: Box<dyn Fn() -> Vec<ContentEntry<T>>>) -> Self
    where
        P: Into<String>,
    {
        Self {
            name: name.into(),
            entries: vec![],
            init_methods: entries,
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

    pub fn into_params<P>(&self, cb: impl Fn(&ContentEntry<T>) -> P) -> Vec<RouteParams>
    where
        P: Into<RouteParams>,
    {
        self.entries.iter().map(cb).map(Into::into).collect()
    }
}

pub trait ContentSourceInternal {
    fn init(&mut self);
    fn get_name(&self) -> &str;
    fn as_any(&self) -> &dyn Any; // Used for type checking at runtime
}

impl<T: 'static> ContentSourceInternal for ContentSource<T> {
    fn init(&mut self) {
        self.entries = (self.init_methods)();
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

enum FrontmatterHolder {
    Yaml(String),
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

        let ast = to_mdast(
            &content,
            &ParseOptions {
                constructs: Constructs {
                    frontmatter: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

        // Check if children
        let children = match ast.children() {
            Some(children) => children,
            None => continue,
        };

        // Check if frontmatter
        let frontmatter: Option<FrontmatterHolder> =
            children.iter().find_map(|child| match child {
                Node::Yaml(frontmatter) => Some(FrontmatterHolder::Yaml(frontmatter.value.clone())),
                _ => None,
            });

        let parsed: Option<T> = frontmatter.map(|FrontmatterHolder::Yaml(frontmatter)| {
            serde_yml::from_str::<T>(&frontmatter).unwrap()
        });

        entries.push(ContentEntry {
            id,
            render: Box::new({
                let content = to_html_with_options(
                    &content,
                    &Options {
                        parse: ParseOptions {
                            constructs: Constructs {
                                frontmatter: true,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        compile: Default::default(),
                    },
                )
                .unwrap();
                move || content.clone()
            }),
            data: parsed.unwrap(),
        });
    }

    entries
}
