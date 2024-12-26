use crate::layout::layout;
use maud::html;
use maudit::page::prelude::*;

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let logo = ctx.assets.add_image("images/logo.svg");

        layout(html! {
            (logo)
            h1 { "Hello World" }
        })
        .into()
    }
}
