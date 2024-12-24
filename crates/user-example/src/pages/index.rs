use maud::html;
use maudit::maudit_macros::route;
use maudit::page::{Page, RenderResult, RouteContext};

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let image = ctx.assets.add_image("data/logo.svg".into());

        let script = ctx.assets.add_script("data/some_other_script.js".into());

        RenderResult::Html(html! {
          h1 { "Index" }
          img src=(image.path.to_string_lossy()) {}
          script src=(script.path.to_string_lossy()) {}
        })
    }
}
