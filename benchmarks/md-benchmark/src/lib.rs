use maudit::{
    content::{glob_markdown, UntypedMarkdownContent},
    content_sources, coronate, routes, BuildOptions,
};
mod page;

pub fn build_website(markdown_count: u32) {
    let _ = coronate(
        routes![page::Article],
        content_sources!["articles" => glob_markdown::<UntypedMarkdownContent>(&format!("content/{}/*.md", markdown_count), None)],
        BuildOptions::default(),
    );
}
