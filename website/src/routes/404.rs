use maud::{html, PreEscaped};
use maudit::route::prelude::*;

use crate::layout::{layout, SeoMeta};

#[route("404.html")]
pub struct NotFound;

impl Route for NotFound {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
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
            Some(SeoMeta {
                title: "404 - Page Not Found".to_string(),
                description: Some(
                    "All the site's a stage, but this page plays not its part.".to_string(),
                ),
                ..Default::default()
            }),
        )
    }
}
