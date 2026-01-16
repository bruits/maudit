use maudit::{
    BuildOptions, PrefetchOptions, PrefetchStrategy,
    content::{UntypedMarkdownContent, glob_markdown},
    content_sources, coronate, routes,
};
mod page;

pub fn build_website(markdown_count: usize) {
    let _ = coronate(
        routes![page::Article],
        content_sources!["articles" => glob_markdown::<UntypedMarkdownContent>(&format!("content/{}/*.md", markdown_count))],
        BuildOptions {
            prefetch: PrefetchOptions {
                // This benchmark is really about testing Maudit's Markdown rendering pipeline, if we enable prefetching then a lot of time
                // is spent in bundling, including the script in pages, etc. instead of that. It's still neat to see how much overhead prefetching adds,
                // but not really in this benchmark.
                strategy: PrefetchStrategy::None,
            },
            ..Default::default()
        },
    );
}
