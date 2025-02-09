mod layout;

use oubli::{forget, routes, BuildOptions, BuildOutput};

mod pages {
    mod index;
    pub use index::Index;
}

pub use pages::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    forget(routes![Index], vec![].into(), BuildOptions::default())
}
