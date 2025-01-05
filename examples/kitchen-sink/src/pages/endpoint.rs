use maudit::page::prelude::*;

#[route("/catalogue/data.json")]
pub struct Endpoint;

impl Endpoint {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let image = ctx.assets.add_image("data/logo.svg");
        let some_script = ctx.assets.add_script("data/script.js");
        ctx.assets.include_style("data/tailwind.css", true);

        // Return some JSON
        RenderResult::Text(format!(
            r#"{{
                    "image": "{}",
                    "script": "{}"
                }}"#,
            image.path.to_string_lossy(),
            some_script.path.to_string_lossy()
        ))
    }
}
