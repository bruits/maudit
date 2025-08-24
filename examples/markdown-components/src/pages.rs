use maud::{html, Markup, PreEscaped, DOCTYPE};
use maudit::content::markdown_entry;
use maudit::page::prelude::*;

#[markdown_entry]
pub struct ComponentExample {
    pub title: String,
    pub description: String,
}

#[route("/")]
pub struct IndexPage;

impl Page<RouteParams, Markup> for IndexPage {
    fn render(&self, ctx: &mut RouteContext) -> Markup {
        let examples = ctx.content.get_source::<ComponentExample>("examples");
        let example = examples.get_entry("showcase");

        // The content is already rendered with the custom components
        // when it was loaded via glob_markdown with options
        let content_html = example.render();

        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (example.data().title) }
                    script src="https://cdn.tailwindcss.com" {}
                    style {
                        r#"
                        .anchor-link { opacity: 0; transition: opacity 0.2s; }
                        h1:hover .anchor-link, h2:hover .anchor-link, h3:hover .anchor-link,
                        h4:hover .anchor-link, h5:hover .anchor-link, h6:hover .anchor-link { opacity: 1; }
                        .external-link::after { content: " ↗"; color: #6366f1; }
                        "#
                    }
                }
                body class="bg-gray-50 min-h-screen" {
                    div class="max-w-4xl mx-auto py-12 px-6" {
                        header class="text-center mb-12" {
                            h1 class="text-4xl font-bold text-gray-900 mb-4" { (example.data().title) }
                            p class="text-xl text-gray-600" { (example.data().description) }
                        }

                        main class="bg-white rounded-lg shadow-lg p-8" {
                            (PreEscaped(content_html))
                        }

                        footer class="mt-12 text-center text-gray-500" {
                            p { "Built with Maudit 🏰 - Custom Markdown Components Example" }
                        }
                    }
                }
            }
        }
    }
}
