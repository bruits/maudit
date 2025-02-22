mod layout;

use oubli::{archetypes, forget, routes, Archetype, BuildOptions, BuildOutput};

mod pages {
    mod index;
    pub use index::Index;
}

pub use pages::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    forget(
        archetypes![("Our blog", blog, Archetype::Blog, "content/blog/*.md"),],
        routes![Index],
        vec![].into(),
        BuildOptions::default(),
    )
}
