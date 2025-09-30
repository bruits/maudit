mod content;
mod layout;
use content::ArticleContent;
use maudit::{BuildOptions, content::glob_markdown, content_sources, routes};

use crate::build::build_website;

mod routes {
    mod article;
    mod index;
    pub use article::Article;
    pub use index::Index;
}

mod build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_website(
        routes![routes::Index],
        content_sources![
            "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
        ],
        &BuildOptions::default(),
    )
}
