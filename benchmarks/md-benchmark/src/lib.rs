use maudit::{
    BuildOptions,
    content::{UntypedMarkdownContent, glob_markdown},
    content_sources, coronate, routes,
};
mod page;

pub fn build_website(markdown_count: usize) {
    let _ = coronate(
        routes![page::Article],
        content_sources!["articles" => glob_markdown::<UntypedMarkdownContent>(&format!("content/{}/*.md", markdown_count))],
        BuildOptions::default(),
    );
}
