use maudit::{
    content::{glob_markdown, UntypedMarkdownContent},
    content_sources, coronate, routes, BuildOptions, BuildOutput,
};
mod page;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    let markdown_count = std::env::var("MARKDOWN_COUNT")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<usize>()
        .unwrap();

    println!("Building with {} markdown files", markdown_count);

    coronate(
        routes![page::Article {
            route: "/yeehaw/[file]".to_string()
        }],
        content_sources!["articles" => glob_markdown::<UntypedMarkdownContent>(&format!("content/{}/*.md", markdown_count))],
        BuildOptions::default(),
    )
}
