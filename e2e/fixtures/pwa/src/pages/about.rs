use maud::html;
use maudit::route::prelude::*;

#[route("/about")]
pub struct About;

impl Route for About {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "PWA Test - About" }
                }
                body {
                    h1 { "About Page" }
                    nav {
                        a href="/" { "Home" }
                    }
                    p id="content" { "This is the about page." }
                }
            }
        })
    }
}
