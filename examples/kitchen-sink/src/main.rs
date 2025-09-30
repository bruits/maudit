use maudit::{AssetsOptions, BuildOptions, BuildOutput, coronate, routes};

mod routes {
    mod dynamic;
    mod endpoint;
    mod index;
    pub use dynamic::DynamicExample;
    pub use endpoint::Endpoint;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![routes::Index, routes::DynamicExample, routes::Endpoint],
        vec![].into(),
        BuildOptions {
            assets: AssetsOptions {
                tailwind_binary_path: "../../node_modules/.bin/tailwindcss".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
