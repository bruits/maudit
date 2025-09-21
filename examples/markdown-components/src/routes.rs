use maud::{html, PreEscaped, DOCTYPE};
use maudit::content::markdown_entry;
use maudit::route::prelude::*;

#[markdown_entry]
pub struct ComponentExample {}

#[route("/")]
pub struct IndexPage;

impl Route for IndexPage {
    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        let examples = ctx.content.get_source::<ComponentExample>("examples");
        let example = examples.get_entry("showcase");

        let content_html = example.render(ctx);
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
        .into()
    }
}
