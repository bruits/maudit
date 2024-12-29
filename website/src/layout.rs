use maud::{html, Markup};
mod docs_sidebar;
mod header;

use docs_sidebar::sidebar;

pub use header::header;
use maudit::generator;
use maudit::page::{RenderResult, RouteContext};

pub fn docs_layout(main: Markup, ctx: &mut RouteContext) -> RenderResult {
    ctx.assets.include_style("assets/prin.css", true);

    layout(
        html! {
            div.container.mx-auto.grid-cols-docs.grid.p-6.py-8 {
                aside {
                    (sidebar(ctx))
                }
                main.w-full.max-w-larger-prose.mx-auto {
                    (main)
                }
            }
        },
        ctx,
    )
}

pub fn layout(main: Markup, ctx: &mut RouteContext) -> RenderResult {
    ctx.assets.include_style("assets/prin.css", true);

    html! {
        head {
            title { "Maudit" }
            (generator())
        }
        body {
            (header(ctx))
            (main)
        }
    }
    .into()
}
