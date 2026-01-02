use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[route(
    "/contact",
    locales(en(prefix = "/en"), sv(prefix = "/sv"), de(path = "/de/kontakt"))
)]
pub struct Contact;

impl Route for Contact {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        layout(html! {
            h1 { "Contact" }
            p { "This route demonstrates different locale syntaxes:" }
            p { "en uses prefix syntax, sv uses prefix syntax, de uses path syntax" }
            p { "Results: /en/contact, /sv/contact, /de/kontakt" }
            nav {
                ul {
                    li { a href="/contact" { "Default" } }
                    li { a href="/en/contact" { "English" } }
                    li { a href="/sv/contact" { "Swedish" } }
                    li { a href="/de/kontakt" { "German" } }
                }
            }
        })
    }
}
