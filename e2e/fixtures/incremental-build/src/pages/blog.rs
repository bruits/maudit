use maud::{html, Markup};
use maudit::route::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[route("/blog")]
pub struct Blog;

impl Route for Blog {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let _style = ctx.assets.add_style("src/assets/blog.css");

        // Generate a unique build ID - uses nanoseconds for uniqueness
        let build_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "0".to_string());

        html! {
            html {
                head {
                    title { "Blog Page" }
                }
                body data-build-id=(build_id) {
                    h1 id="title" { "Blog" }
                    p id="content" { "Read our latest posts" }
                }
            }
        }
    }
}
