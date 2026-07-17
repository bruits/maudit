use maudit::route::prelude::*;

use maud::html;

use super::dynamic::{DynamicExample, Params as DynamicExampleParams};

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let image = ctx.assets.add_image("data/logo.svg")?;
        let script = ctx.assets.add_script("data/some_other_script.js")?;
        let style = ctx
            .assets
            .add_style_with_options("data/tailwind.css", StyleOptions { tailwind: true })?;

        let og_image = ctx.assets.add_opengraph_image(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
                    <rect width="100%" height="100%" fill="#1a1a1a"/>
                    <text x="60" y="330" fill="#ffffff" font-size="72" font-family="sans-serif">Kitchen Sink</text>
                </svg>"##,
        )?;

        let link_to_first_dynamic = DynamicExample.url(DynamicExampleParams { page: 1 });

        Ok(html! {
            head {
                title { "Index" }
                link rel="stylesheet" href=(style.url()) {}
                (og_image.render())
            }
            h1 { "Index" }
            img src=(image.url()) {}
            script src=(script.url()) {}
            (image.render("Maudit logo, a crudely drawn crown"))
            a."text-red-500" href=(link_to_first_dynamic) { "Go to first dynamic page" }
        })
    }
}
