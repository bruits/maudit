use crate::layout::layout;
use maud::html;
use maudit::{assets::ImageOptions, route::prelude::*};

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let logo = ctx.assets.add_image("images/logo.svg")?;
        let walrus = ctx.assets.add_image_with_options(
            "images/walrus.jpg",
            ImageOptions {
                width: Some(200),
                height: Some(200),
                format: Some(maudit::assets::ImageFormat::WebP),
            },
        )?;

        Ok(layout(html! {
            (logo.render("Maudit logo, a crudely drawn crown"))
            h1 { "Hello World" }
            h2 { "Here's a 200x200 walrus:" }
            (walrus.render("A walrus with tusks"))
        }))
    }
}
