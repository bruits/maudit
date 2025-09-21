use maud::{html, PreEscaped};
use maudit::page::prelude::*;

use crate::layout::layout;

#[route("404.html")]
pub struct NotFound;

impl Route for NotFound {
    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        layout(
            html! {
                div.container.mx-auto.text-center.my-50.flex.items-center.flex-col."gap-y-4"."px-8"."sm:px-0" {
                    (PreEscaped(include_str!("../../assets/logo.svg")))
                    h1.text-6xl { "404 - Not Found" }
                    p.text-xl { "All the site's a stage, but this page plays not its part." }
                    a.btn.btn-primary href="/" { "Go back to safety" }
                }
            },
            false,
            false,
            ctx,
        )
    }
}
