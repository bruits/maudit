use maud::{html, Markup};
mod header;

pub use header::header;
use maudit::generator;
use maudit::page::{RenderResult, RouteContext};

pub fn layout(main: Markup, ctx: &mut RouteContext) -> RenderResult {
    ctx.assets.include_style("assets/prin.css", true);

    html! {
        head {
            title { "Maudit" }
            (generator())
        }
        body {
            (main)
        }
    }
    .into()
}
