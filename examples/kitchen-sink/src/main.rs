use maudit::{coronate, routes, BuildOptions, BuildOutput};

mod pages {
    mod dynamic;
    mod endpoint;
    mod index;
    pub use dynamic::DynamicExample;
    pub use endpoint::Endpoint;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::Index, pages::DynamicExample, pages::Endpoint],
        vec![].into(),
        BuildOptions {
            tailwind_binary_path: "../../node_modules/.bin/tailwindcss".to_string(),
            ..Default::default()
        },
    )
}
