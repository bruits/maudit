use maud::html;
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "Test Site - Home" }
                }
                body {
                    h1 { "Home Page" }
                    nav {
                        ul {
                            li {
                                a href="/about" { "About" }
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
                        p { "Welcome to the test site!" }
                    }
                }
            }
        })
    }
}
