use maud::{html, Markup};
use maudit::route::prelude::*;

#[route("/blog")]
pub struct Blog;

impl Route for Blog {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let _style = ctx.assets.add_style("src/assets/blog.css");

        html! {
            html {
                head {
                    title { "Blog Page" }
                }
                body {
                    h1 id="title" { "Blog" }
                    p id="content" { "Read our latest posts" }
                }
            }
        }
    }
}
