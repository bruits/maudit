use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[locales(en(path = "/en"), sv(path = "/sv"), de(path = "/de"))]
#[route]
pub struct Index;

impl Route for Index {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        layout(html! {
            h1 { "i18n Example" }
            p { "This route only exists as variants - no base path!" }
            nav {
                ul {
                    li { a href="/en" { "English" } }
                    li { a href="/sv" { "Swedish" } }
                    li { a href="/de" { "German" } }
                }
            }
        })
    }
}
