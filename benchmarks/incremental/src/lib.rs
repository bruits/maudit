use maudit::{BuildOptions, PrefetchOptions, PrefetchStrategy, content_sources, coronate, routes};
mod page;

pub fn build_website() {
    let _ = coronate(
        routes![page::Article],
        content_sources![],
        BuildOptions {
            prefetch: PrefetchOptions {
                strategy: PrefetchStrategy::None,
                ..Default::default()
            },
            incremental: true,
            ..Default::default()
        },
    );
}
