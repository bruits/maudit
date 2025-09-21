use crate::layout::layout;
use maud::html;
use maudit::page::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        let logo = ctx.assets.add_image("images/logo.svg");

        let archetype_store = ctx
            .content
            .get_source::<oubli::ArchetypeStoreEntry>("archetype_store");

        layout(html! {
            (logo)
            h1 { "Hello World" }
            @for archetype in &archetype_store.entries {
                a href=(archetype.id) { (archetype.data(ctx).title) }
            }
        })
        .into()
    }
}
