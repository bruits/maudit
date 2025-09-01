use maudit::{page::prelude::*, StyleOptions};

use maud::html;

use super::dynamic::{DynamicExample, Params as DynamicExampleParams};

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let image = ctx.assets.add_image("data/logo.svg");
        let script = ctx.assets.add_script("data/some_other_script.js");
        let style = ctx
            .assets
            .add_style("data/tailwind.css", Some(StyleOptions { tailwind: true }));

        let link_to_first_dynamic =
            get_page_url(&DynamicExample, &DynamicExampleParams { page: 1 });

        html! {
            head {
                title { "Index" }
                link rel="stylesheet" href=(style.url().unwrap()) {}
            }
            h1 { "Index" }
            img src=(image.url().unwrap()) {}
            script src=(script.url().unwrap()) {}
            a."text-red-500" href=(link_to_first_dynamic) { "Go to first dynamic page" }
        }
        .into()
    }
}
