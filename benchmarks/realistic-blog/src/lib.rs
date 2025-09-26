mod content;
mod layout;

use content::ArticleContent;
use maudit::{
    content::{glob_markdown_with_options, shortcodes::MarkdownShortcodes, MarkdownOptions},
    content_sources, coronate, routes, BuildOptions,
};

mod routes {
    mod article;
    mod index;
    pub use article::{Article, Articles};
    pub use index::Index;
}

pub fn build_website() {
    let _ = coronate(
        routes![routes::Index, routes::Articles, routes::Article],
        content_sources![
            "articles" => glob_markdown_with_options::<ArticleContent>("content/articles/*.md", MarkdownOptions {
                shortcodes: {
                    let mut shortcodes = MarkdownShortcodes::default();

                    shortcodes.register("youtube", |attrs, _| {
                        if let Some(id) = attrs.get::<String>("id") {
                            format!(r#"<iframe width="560" height="315" src="https://www.youtube.com/embed/{}" frameborder="0" allowfullscreen></iframe>"#, id)
                        } else {
                            panic!("YouTube shortcode requires an 'id' attribute");
                        }
                    });

                    shortcodes
                },
            ..Default::default()
            })
        ],
        BuildOptions::default(),
    );
}
