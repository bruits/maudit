use maud::html;
use maudit::route::prelude::*;

#[route("/blog")]
pub struct Blog;

impl Route for Blog {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "Test Site - Blog" }
                }
                body {
                    h1 { "Blog Page" }
                    nav {
                        ul {
                            li {
                                a href="/" { "Home" }
                            }
                            li {
                                a href="/about" { "About" }
                            }
                            li {
                                a href="/contact" { "Contact" }
                            }
                        }
                    }
                    div id="content" {
                        p { "This is the blog page." }
                    }
                }
            }
        })
    }
}
