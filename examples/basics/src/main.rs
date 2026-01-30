mod layout;

use maudit::{BuildOptions, BuildOutput, content_sources, coronate, routes};

mod routes {
    mod another;
    mod index;
    pub use another::Another;
    pub use index::Index;
}

pub use routes::Another;
pub use routes::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![Index, Another],
        content_sources![],
        BuildOptions::default(),
    )
}
