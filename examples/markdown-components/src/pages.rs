use maud::{html, Markup, PreEscaped, DOCTYPE};
use maudit::content::markdown_entry;
use maudit::page::prelude::*;

#[markdown_entry]
pub struct ComponentExample {}

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
                    title { "Custom Markdown Components Showcase" }
                    style {
                       (PreEscaped(include_str!("./style.css")))
                    }
                }
                body {
                    div class="container" {
                        (PreEscaped(content_html))
                    }
                }
            }
        }
    }
}
