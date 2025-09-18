use maudit::content::markdown_entry;

#[markdown_entry]
pub struct ArticleContent {
    pub title: String,
    pub description: String,
}
