use maudit::content::{glob_markdown, markdown_entry};

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct ArticleContent {
    pub title: String,
    pub description: String,
}

pub fn load_articles() -> Vec<maudit::content::Entry<ArticleContent>> {
    glob_markdown("content/articles/*.md")
}
