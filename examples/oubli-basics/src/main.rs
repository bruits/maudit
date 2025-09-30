mod layout;

use oubli::{Archetype, BuildOptions, BuildOutput, archetypes, content_sources, forget, routes};

mod routes {
    mod index;
    pub use index::Index;
}

pub use routes::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    forget(
        archetypes![("Our blog", blog, Archetype::Blog, "content/blog/*.md"),],
        routes![Index],
        content_sources![],
        BuildOptions::default(),
    )
}
