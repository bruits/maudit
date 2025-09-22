use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let logo = ctx.assets.add_image("images/logo.svg");

        layout(html! {
            (logo)
            h1 { "Hello World" }
        })
    }
}
