use content::content_sources;
use maudit::{AssetsOptions, BuildOptions, BuildOutput, PrefetchOptions, coronate, routes};

mod content;
mod layout;
mod routes;

use routes::*;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![
            Index,
            DocsIndex,
            NewsIndex,
            DocsPage,
            NewsPage,
            ChatRedirect,
            NotFound,
            Contribute
        ],
        content_sources(),
        BuildOptions {
            base_url: Some("https://maudit.org".to_string()),
            assets: AssetsOptions {
                tailwind_binary_path: "../node_modules/.bin/tailwindcss".into(),
                ..Default::default()
            },
            prefetch: PrefetchOptions {
                strategy: maudit::PrefetchStrategy::Hover,
            },
            ..Default::default()
        },
    )
}
