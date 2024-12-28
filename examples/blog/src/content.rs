use maudit::content::{glob_markdown, ContentSource, ContentSources};
use maudit::content_sources;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ArticleContent {
    pub title: String,
    pub description: String,
}

pub fn content_sources() -> ContentSources {
    content_sources!(
        ContentSource::new(
            "articles",
            glob_markdown::<ArticleContent>("content/articles/*.md")
        ),
        ContentSource::new(
            "authors",
            glob_markdown::<ArticleContent>("content/authors/*.md")
        )
    )
}
