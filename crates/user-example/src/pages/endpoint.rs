use maudit::maudit_macros::route;
use maudit::page::{Page, RenderResult, RouteContext};

#[route("/catalogue/data.json")]
pub struct Endpoint;

impl Page for Endpoint {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let image = ctx.assets.add_image("data/logo.svg".into());

        // Return some JSON
        RenderResult::Text(format!(
            r#"{{
								"image": "{}"
								}}"#,
            image.path.to_string_lossy()
        ))
    }
}
