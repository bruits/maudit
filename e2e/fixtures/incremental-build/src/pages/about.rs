use maud::html;
use maudit::route::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[route("/about")]
pub struct About;

impl Route for About {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let _image = ctx.assets.add_image("src/assets/team.png");
        let _script = ctx.assets.add_script("src/assets/about.js");
        // Shared style with index page (for testing shared assets)
        let _style = ctx.assets.add_style("src/assets/styles.css");

        // Generate a unique build ID - uses nanoseconds for uniqueness
        let build_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "0".to_string());

        html! {
            html {
                head {
                    title { "About Page" }
                }
                body data-build-id=(build_id) {
                    h1 id="title" { "About Us" }
                    p id="content" { "Learn more about us" }
                }
            }
        }
    }
}
