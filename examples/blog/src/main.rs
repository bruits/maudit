mod content;
mod layout;
use content::ArticleContent;
use maudit::{
    BuildOptions, BuildOutput, content::glob_markdown, content_sources, coronate, routes,
};

mod routes {
    mod article;
    mod feed;
    mod index;
    pub use article::Article;
    pub use article::ArticleParams;
    pub use feed::Feed;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![routes::Index, routes::Article, routes::Feed],
        content_sources![
            "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
        ],
        BuildOptions::default(),
    )
}
