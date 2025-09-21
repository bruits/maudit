mod layout;

use maudit::{coronate, routes, BuildOptions, BuildOutput};

mod routes {
    mod index;
    pub use index::Index;
}

pub use routes::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(routes![Index], vec![].into(), BuildOptions::default())
}
