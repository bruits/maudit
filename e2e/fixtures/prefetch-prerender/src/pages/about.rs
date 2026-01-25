use maud::html;
use maudit::route::prelude::*;

#[route("/about")]
pub struct About;

impl Route for About {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "Test Site - About" }
                }
                body {
                    h1 { "About Page" }
                    nav {
                        ul {
                            li {
                                a href="/" { "Home" }
                            }
                            li {
                                a href="/contact" { "Contact" }
                            }
                            li {
                                a href="/blog" { "Blog" }
                            }
                        }
                    }
                    div id="content" {
                        p { "This is the about page." }
                    }
                }
            }
        })
    }
}
