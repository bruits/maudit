use maud::html;
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(html! {
            html {
                head {
                    title { "Hot Reload Test" }
                }
                body {
                    h1 id="title" { "Original Title" }
                    div id="content" {
                        p id="message" { "Original message" }
                        ul id="list" {
                            li { "Item 1" }
                            li { "Item 2" }
                        }
                    }
                    footer {
                        p { "Footer content" }
                    }
                }
            }
        })
    }
}
