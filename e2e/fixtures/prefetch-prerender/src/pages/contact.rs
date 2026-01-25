use maud::html;
use maudit::route::prelude::*;

#[route("/contact")]
pub struct Contact;

impl Route for Contact {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "Test Site - Contact" }
                }
                body {
                    h1 { "Contact Page" }
                    nav {
                        ul {
                            li {
                                a href="/" { "Home" }
                            }
                            li {
                                a href="/about" { "About" }
                            }
                            li {
                                a href="/blog" { "Blog" }
                            }
                        }
                    }
                    div id="content" {
                        p { "This is the contact page." }
                    }
                }
            }
        })
    }
}
