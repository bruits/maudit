use maud::{PreEscaped, Render};
use maudit::content::{glob_markdown, markdown_entry, ContentSources};
use maudit::content_sources;
use serde::Deserialize;

#[derive(Deserialize, Eq, PartialEq, PartialOrd, Hash, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DocsSection {
    GettingStarted,
    CoreConcepts,
    Advanced,
}

impl Render for DocsSection {
    fn render(&self) -> PreEscaped<String> {
        match self {
            DocsSection::GettingStarted => PreEscaped("Getting Started".to_string()),
            DocsSection::CoreConcepts => PreEscaped("Core Concepts".to_string()),
            DocsSection::Advanced => PreEscaped("Advanced".to_string()),
        }
    }
}

#[markdown_entry]
pub struct DocsContent {
    pub title: String,
    pub description: Option<String>,
    pub section: Option<DocsSection>,
}

#[markdown_entry]
pub struct NewsContent {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<String>,
    pub author: Option<String>,
}

pub fn content_sources() -> ContentSources {
    content_sources!["docs" => glob_markdown::<DocsContent>("content/docs/*.md", None), "news" => glob_markdown::<NewsContent>("content/news/*.md", None)]
}
