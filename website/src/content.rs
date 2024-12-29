use maud::{PreEscaped, Render};
use maudit::content::{glob_markdown, ContentSource, ContentSources};
use maudit::content_sources;
use serde::Deserialize;

#[derive(Deserialize, Eq, PartialEq, PartialOrd, Hash, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DocsSection {
    GettingStarted,
}

impl Render for DocsSection {
    fn render(&self) -> PreEscaped<String> {
        match self {
            DocsSection::GettingStarted => PreEscaped("Getting Started".to_string()),
        }
    }
}

#[derive(Deserialize)]
pub struct DocsContent {
    pub title: String,
    pub description: String,
    pub section: Option<DocsSection>,
}

pub fn content_sources() -> ContentSources {
    content_sources!(ContentSource::new(
        "docs",
        glob_markdown::<DocsContent>("content/docs/*.md")
    ))
}
