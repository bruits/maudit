use maudit::{
    content::{glob_markdown, UntypedMarkdownContent},
    content_sources, coronate, routes, BuildOptions,
};
mod page;

pub fn build_website(markdown_count: usize) {
    let _ = coronate(
        routes![page::Article],
        content_sources!["articles" => glob_markdown::<UntypedMarkdownContent>(&format!("content/{}/*.md", markdown_count))],
        BuildOptions::default(),
    );
}
