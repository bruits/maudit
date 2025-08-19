use maud::html;
use maudit::page::prelude::*;

use crate::layout::layout;

#[route("404.html")]
pub struct NotFound;

impl Page for NotFound {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        layout(
            html! {
                div class="container" {
                    h1 { "404 - Not Found" }
                    p { "The page you are looking for could not be found." }
                }
            },
            false,
            false,
            ctx,
        )
    }
}
