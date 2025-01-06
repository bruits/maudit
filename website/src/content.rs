use maud::{PreEscaped, Render};
use maudit::content::{glob_markdown, markdown_entry, ContentSources};
use maudit::content_sources;
use serde::Deserialize;

#[derive(Deserialize, Eq, PartialEq, PartialOrd, Hash, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DocsSection {
    GettingStarted,
    CoreConcepts,
}

impl Render for DocsSection {
    fn render(&self) -> PreEscaped<String> {
        match self {
            DocsSection::GettingStarted => PreEscaped("Getting Started".to_string()),
            DocsSection::CoreConcepts => PreEscaped("Core Concepts".to_string()),
        }
    }
}

#[markdown_entry]
pub struct DocsContent {
    pub title: String,
    pub description: Option<String>,
    pub section: Option<DocsSection>,
}

pub fn content_sources() -> ContentSources {
    content_sources!["docs" => glob_markdown::<DocsContent>("content/docs/*.md")]
}
