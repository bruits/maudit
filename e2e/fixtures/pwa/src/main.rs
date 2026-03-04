use maudit::{BuildOptions, BuildOutput, PwaOptions, content_sources, coronate, routes};

mod pages {
    mod about;
    mod index;
    pub use about::About;
    pub use index::Index;
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![pages::Index, pages::About],
        content_sources![],
        BuildOptions {
            pwa: PwaOptions {
                enabled: true,
                name: "PWA Test App".into(),
                short_name: Some("PWA Test".into()),
                precache: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
