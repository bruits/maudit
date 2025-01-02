use serde::Deserialize;

#[derive(Deserialize)]
pub struct ArticleContent {
    pub title: String,
    pub description: String,
}
