use content::content_sources;
use maudit::{coronate, routes, BuildOptions, BuildOutput};

mod content;
mod layout;
mod pages;

use pages::{ChatRedirect, DocsIndex, DocsPage, Index};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![Index, DocsIndex, DocsPage, ChatRedirect],
        content_sources(),
        BuildOptions::default(),
    )
}
