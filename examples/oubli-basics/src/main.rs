mod layout;

use oubli::{archetypes, forget, routes, Archetype, BuildOptions, BuildOutput};

mod routes {
    mod index;
    pub use index::Index;
}

pub use routes::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    forget(
        archetypes![("Our blog", blog, Archetype::Blog, "content/blog/*.md"),],
        routes![Index],
        vec![].into(),
        BuildOptions::default(),
    )
}
