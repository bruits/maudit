use maud::{html, Markup, DOCTYPE};
mod docs_sidebars;
mod header;

use docs_sidebars::{left_sidebar, right_sidebar};

pub use header::header;
use maudit::generator;
use maudit::page::{RenderResult, RouteContext};

pub fn docs_layout(main: Markup, ctx: &mut RouteContext) -> RenderResult {
    ctx.assets.include_style("assets/prin.css", true);

    layout(
        html! {
            div.container.mx-auto.grid-cols-docs.grid."min-h-[calc(100%-64px)]" {
                aside.bg-gradient-to-l."from-darker-white"."py-8"."h-full".border-r.border-r-borders {
                    (left_sidebar(ctx))
                }
                main.w-full.max-w-larger-prose.mx-auto.py-8 {
                    (main)
                }
                aside."py-8" {
                    (right_sidebar(ctx))
                }
            }
        },
        true,
        ctx,
    )
}

pub fn layout(main: Markup, bottom_border: bool, ctx: &mut RouteContext) -> RenderResult {
    ctx.assets.include_style("assets/prin.css", true);

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Maudit" }
                (generator())
                link rel="icon" href="/favicon.svg";
            }
            body {
                (header(ctx, bottom_border))
                (main)
                footer.bg-our-black.text-white {
                    div.container.mx-auto.py-8 {
                        p.text-center.text-sm.italic { "Maudit" }
                    }
                }
            }
        }
    }
    .into()
}
