use content::content_sources;
use maudit::{coronate, routes, AssetsOptions, BuildOptions, BuildOutput};

mod content;
mod layout;
mod pages;

use pages::*;

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
            assets: AssetsOptions {
                tailwind_binary_path: "../node_modules/.bin/tailwindcss".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
