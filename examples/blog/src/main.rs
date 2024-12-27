mod content;
mod layout;
use content::ArticleContent;
use maudit::{
    content::{glob_markdown, ContentSource, ContentSources},
    coronate, generate_pages_mod, routes, BuildOptions, BuildOutput,
};

generate_pages_mod!();

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![Index, Article],
        ContentSources(vec![Box::new(ContentSource {
            name: "articles".to_string(),
            entries: glob_markdown::<ArticleContent>("content/articles/*.md"),
        })]),
        BuildOptions::default(),
    )
}
