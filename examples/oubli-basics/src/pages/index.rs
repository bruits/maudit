use crate::layout::layout;
use maud::html;
use maudit::page::prelude::*;

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let logo = ctx.assets.add_image("images/logo.svg");

        let data_store = ctx
            .content
            .get_source::<oubli::DataStoreEntry>("data_store");

        layout(html! {
            (logo)
            h1 { "Hello World" }
            @for entry in &data_store.entries {
                a href=(entry.id) { (entry.id) }
            }
        })
        .into()
    }
}
