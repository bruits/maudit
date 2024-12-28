mod content;
mod layout;
use content::content_sources;
use maudit::{coronate, routes, BuildOptions, BuildOutput};

mod pages {
    mod article;
    mod index;
    pub use article::Article;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::Index, pages::Article],
        content_sources(),
        BuildOptions::default(),
    )
}
