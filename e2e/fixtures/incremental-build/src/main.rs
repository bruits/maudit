use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

mod content;
mod pages;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![
            pages::index::Index,
            pages::about::About,
            pages::blog::Blog,
            pages::articles::Articles,
            pages::article::Article
        ],
        content_sources![
            "articles" => content::load_articles()
        ],
        BuildOptions::default(),
    )
}
