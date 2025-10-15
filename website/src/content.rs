use chrono::NaiveDate;
use maud::{PreEscaped, Render};
use maudit::content::{
    ContentSources, MarkdownOptions, glob_markdown, glob_markdown_with_options, markdown_entry,
};
use maudit::content_sources;
use serde::Deserialize;

#[derive(Deserialize, Eq, PartialEq, PartialOrd, Hash, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DocsSection {
    GettingStarted,
    CoreConcepts,
    Guide,
    Advanced,
}

impl Render for DocsSection {
    fn render(&self) -> PreEscaped<String> {
        match self {
            DocsSection::GettingStarted => PreEscaped("Getting Started".to_string()),
            DocsSection::CoreConcepts => PreEscaped("Core Concepts".to_string()),
            DocsSection::Guide => PreEscaped("Guide".to_string()),
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
    pub date: NaiveDate,
    pub author: Option<String>,
}

pub fn content_sources() -> ContentSources {
    content_sources!["docs" => glob_markdown_with_options::<DocsContent>("content/docs/*.md", MarkdownOptions {
        highlight_theme: "base16-eighties.dark".into(),
        ..Default::default()
    }), "news" => glob_markdown::<NewsContent>("content/news/*.md")]
}
