use maudit::{coronate, routes, BuildOptions, BuildOutput};

mod layout;
mod pages {
    mod index;
    pub use index::Index;
}

pub use pages::Index;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(routes![Index], vec![].into(), BuildOptions::default())
}
