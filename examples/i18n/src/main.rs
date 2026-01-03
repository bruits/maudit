mod layout;
mod routes;

use maudit::{BuildOptions, BuildOutput, content_sources, coronate, routes};
use routes::{About, Article, Contact, Index};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![Index, About, Contact, Article],
        content_sources![],
        BuildOptions::default(),
    )
}
