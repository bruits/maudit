use maudit::page::prelude::*;

use maud::html;

use super::dynamic::{DynamicExample, Params as DynamicExampleParams};

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let image = ctx.assets.add_image("data/logo.svg");
        let script = ctx.assets.add_script("data/some_other_script.js");
        let style = ctx.assets.add_style("data/tailwind.css", true);

        let link_to_first_dynamic = DynamicExample::url_unsafe(&DynamicExampleParams { page: 1 });

        let safe_link_to_first_dynamic = DynamicExample
            .url(
                &DynamicExampleParams { page: 0 },
                &DynamicRouteContext {
                    content: ctx.content,
                },
            )
            .unwrap();

        html! {
            head {
                title { "Index" }
                link rel="stylesheet" href=(style.url().unwrap()) {}
            }
            h1 { "Index" }
            img src=(image.url().unwrap()) {}
            script src=(script.url().unwrap()) {}
            a."text-red-500" href=(link_to_first_dynamic) { "Go to first dynamic page" }
            a href=(safe_link_to_first_dynamic) { "Go to first dynamic page (safe)" }
        }
        .into()
    }
}
