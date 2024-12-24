use maudit::page::prelude::*;

use maud::html;

use super::dynamic::{DynamicExample, Params as DynamicExampleParams};

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let image = ctx.assets.add_image("data/logo.svg".into());
        let script = ctx.assets.add_script("data/some_other_script.js".into());

        let link_to_first_dynamic = DynamicExample::url(&DynamicExampleParams { page: 1 });

        println!("Link to endpoint: {}", link_to_first_dynamic);

        RenderResult::Html(html! {
          h1 { "Index" }
          img src=(image.path.to_string_lossy()) {}
          script src=(script.path.to_string_lossy()) {}
        })
    }
}
