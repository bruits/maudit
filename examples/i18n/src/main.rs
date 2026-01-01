mod layout;
mod routes;

use maudit::{BuildOptions, BuildOutput, content_sources, coronate, routes};
use routes::{About, Article, Index, Mixed};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![Index, About, Article, Mixed],
        content_sources![],
        BuildOptions::default(),
    )
}
