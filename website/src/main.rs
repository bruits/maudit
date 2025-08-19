use content::content_sources;
use maudit::{coronate, routes, BuildOptions, BuildOutput};

mod content;
mod layout;
mod pages;

use pages::{ChatRedirect, DocsIndex, DocsPage, Index, NewsIndex, NewsPage};

use crate::pages::NotFound;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![
            Index,
            DocsIndex,
            NewsIndex,
            DocsPage,
            NewsPage,
            ChatRedirect,
            NotFound
        ],
        content_sources(),
        BuildOptions {
            tailwind_binary_path: "../node_modules/.bin/tailwindcss".to_string(),
            ..Default::default()
        },
    )
}
