use maudit::route::prelude::*;

#[route("/catalogue/data.json")]
pub struct Endpoint;

impl Route for Endpoint {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let image = ctx.assets.add_image("data/logo.svg")?;
        let some_script = ctx.assets.add_script("data/script.js")?;
        ctx.assets
            .include_style_with_options("data/tailwind.css", StyleOptions { tailwind: true })?;

        // Return some JSON
        Ok(format!(
            r#"{{
                    "image": "{}",
                    "script": "{}"
                }}"#,
            image.path.to_string_lossy(),
            some_script.path.to_string_lossy()
        ))
    }
}
