use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

// Demonstrates all three locale syntax options:
// 1. Shorthand full path: en = "..."
// 2. Explicit full path: sv(path = "...")
// 3. Prefix: de(prefix = "...")
#[route(
    "/about",
    locales(
        en = "/en/about",                  // Shorthand full path
        sv(path = "/sv/om-oss"),           // Explicit full path
        de(prefix = "/de")                 // Prefix (becomes /de/about)
    )
)]
pub struct About;

impl Route for About {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        layout(html! {
            h1 { "About" }
            p { "This route demonstrates all locale syntax variations:" }
            ul {
                li { code { "en = \"/en/about\"" } " - shorthand full path" }
                li { code { "sv(path = \"/sv/om-oss\")" } " - explicit full path" }
                li { code { "de(prefix = \"/de\")" } " - prefix (becomes /de/about)" }
            }
            nav {
                h3 { "Generated routes:" }
                ul {
                    li { a href="/about" { "Default (/about)" } }
                    li { a href="/en/about" { "English (/en/about)" } }
                    li { a href="/sv/om-oss" { "Swedish (/sv/om-oss)" } }
                    li { a href="/de/about" { "German (/de/about)" } }
                }
            }
        })
    }
}
