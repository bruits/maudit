mod layout;

use maudit::{BuildOptions, BuildOutput, content_sources, coronate, routes};

mod routes {
    mod index;
    pub use index::Index;
}

pub use routes::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(routes![Index], content_sources![], BuildOptions::default())
}
