use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

mod pages;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::index::Index, pages::about::About, pages::blog::Blog],
        content_sources![],
        BuildOptions::default(),
    )
}
