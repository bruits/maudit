use content::content_sources;
use graphgarden_core::{
    build::build,
    config::{Config, OutputConfig, ParseConfig, SiteConfig},
};
use maudit::{AssetsOptions, BuildOptions, BuildOutput, coronate, routes};

mod content;
mod layout;
mod routes;

use routes::*;

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    let output = coronate(
        routes![
            Index,
            DocsIndex,
            NewsIndex,
            DocsPage,
            NewsPage,
            ChatRedirect,
            NotFound,
            Contribute
        ],
        content_sources(),
        BuildOptions {
            base_url: Some("https://maudit.org".to_string()),
            assets: AssetsOptions {
                tailwind_binary_path: "../node_modules/.bin/tailwindcss".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    let gg_config = Config {
        site: SiteConfig {
            base_url: "https://maudit.org/".into(),
            title: "Maudit".into(),
            description: Some("A Rust library for building static websites".into()),
            language: Some("en".into()),
        },
        friends: vec![
            "https://erika.florist/".into(),
            "https://goulven-clech.dev/".into(),
            "https://bruits.org/".into(),
        ],
        output: OutputConfig {
            dir: "./dist".into(),
        },
        parse: ParseConfig {
            exclude_selectors: Some(vec![
                "header".into(),
                "footer".into(),
                "nav".into(),
                "[data-graphgarden-ignore]".into(),
            ]),
            exclude: Some(vec!["404.html".into()]),
            ..Default::default()
        },
    };

    let public_file = build(&gg_config)?;
    let json = public_file.to_json()?;

    let well_known_dir = std::path::Path::new("./dist/.well-known");
    std::fs::create_dir_all(well_known_dir)?;
    std::fs::write(well_known_dir.join("graphgarden.json"), json)?;

    Ok(output)
}
