use maudit::{BuildOptions, content_sources, coronate, routes};
mod page;

pub fn build_website() {
    let _ = coronate(
        routes![page::Article],
        content_sources![],
        BuildOptions::default(),
    );
}
