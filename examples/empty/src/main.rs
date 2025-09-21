use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

mod routes {
    mod index;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![routes::Index],
        content_sources![],
        BuildOptions::default(),
    )
}
