use maudit::{BuildOptions, BuildOutput, content_sources, coronate, routes};

mod pages;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::index::Index, pages::about::About, pages::blog::Blog],
        content_sources![],
        BuildOptions::default(),
    )
}
