mod content;
mod layout;
use content::ArticleContent;
use maudit::{
    content::glob_markdown, content_sources, coronate, routes, BuildOptions, BuildOutput,
};

mod pages {
    mod article;
    mod index;
    pub use article::Article;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::Index, pages::Article],
        content_sources![
            "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
        ],
        BuildOptions::default(),
    )
}
