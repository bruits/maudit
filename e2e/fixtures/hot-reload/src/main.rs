use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

mod pages {
    mod index;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::Index],
        content_sources![],
        BuildOptions::default(),
    )
}
