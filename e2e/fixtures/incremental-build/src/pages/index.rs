use maud::{html, Markup};
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let _image = ctx.assets.add_image("src/assets/logo.png");
        let _script = ctx.assets.add_script("src/assets/main.js");
        let _style = ctx.assets.add_style("src/assets/styles.css");

        html! {
            html {
                head {
                    title { "Home Page" }
                }
                body {
                    h1 id="title" { "Home Page" }
                    p id="content" { "Welcome to the home page" }
                }
            }
        }
    }
}
