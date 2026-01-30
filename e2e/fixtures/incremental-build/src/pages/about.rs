use maud::{html, Markup};
use maudit::route::prelude::*;

#[route("/about")]
pub struct About;

impl Route for About {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let _image = ctx.assets.add_image("src/assets/team.png");
        let _script = ctx.assets.add_script("src/assets/about.js");

        html! {
            html {
                head {
                    title { "About Page" }
                }
                body {
                    h1 id="title" { "About Us" }
                    p id="content" { "Learn more about us" }
                }
            }
        }
    }
}
