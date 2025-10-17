use content::content_sources;
use maudit::{AssetsOptions, BuildOptions, BuildOutput, coronate, routes};

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
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
