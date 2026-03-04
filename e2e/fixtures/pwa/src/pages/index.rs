use maud::html;
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "PWA Test - Home" }
                }
                body {
                    h1 { "Home Page" }
                    nav {
                        a href="/about" { "About" }
                    }
                    p id="content" { "Welcome to the PWA test site!" }
                }
            }
        })
    }
}
