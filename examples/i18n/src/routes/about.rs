use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[locales(
    en(path = "/en/about"),
    sv(path = "/sv/om-oss"),
    de(path = "/de/uber-uns")
)]
#[route("/about")]
pub struct About;

impl Route for About {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        layout(html! {
            h1 { "About" }
            p { "This route has both a base path and localized variants." }
            nav {
                ul {
                    li { a href="/about" { "Default" } }
                    li { a href="/en/about" { "English" } }
                    li { a href="/sv/om-oss" { "Swedish" } }
                    li { a href="/de/uber-uns" { "German" } }
                }
            }
        })
    }
}
