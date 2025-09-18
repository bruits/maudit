mod content;
mod layout;
use content::ArticleContent;
use maudit::{content::glob_markdown, content_sources, routes, BuildOptions};

use crate::build::build_website;

mod pages {
    mod article;
    mod index;
    pub use article::Article;
    pub use index::Index;
}

mod build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_website(
        routes![pages::Index],
        content_sources![
            "articles" => glob_markdown::<ArticleContent>("content/articles/*.md", None)
        ],
        BuildOptions::default(),
    )
}
