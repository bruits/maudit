use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[route(locales(en = "/en", sv = "/sv", de = "/de"))]
pub struct Index;

impl Route for Index {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        layout(html! {
            h1 { "i18n Example" }
            p { "This route only exists as variants - no base path!" }
            p { "The current variant is: " (if let Some(variant) = &_ctx.variant {
                variant
            } else {
                "none"
            }) }
            nav {
                ul {
                    li { a href="/en/" { "English" } }
                    li { a href="/sv/" { "Swedish" } }
                    li { a href="/de/" { "German" } }
                }
            }
        })
    }
}
