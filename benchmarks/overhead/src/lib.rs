use maudit::{BuildOptions, PrefetchOptions, PrefetchStrategy, content_sources, coronate, routes};
mod page;

pub fn build_website() {
    let _ = coronate(
        routes![page::Article],
        content_sources![],
        BuildOptions {
            prefetch: PrefetchOptions {
                // This benchmark is really about testing Maudit's overhead, if we enable prefetching then a lot of time
                // is spent in bundling, including the script in pages, etc. instead of Maudit itself.
                strategy: PrefetchStrategy::None,
                ..Default::default()
            },
            ..Default::default()
        },
    );
}
