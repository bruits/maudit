use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

mod pages {
    mod about;
    mod blog;
    mod contact;
    mod index;
    pub use about::About;
    pub use blog::Blog;
    pub use contact::Contact;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::Index, pages::About, pages::Contact, pages::Blog],
        content_sources![],
        BuildOptions::default(),
    )
}
