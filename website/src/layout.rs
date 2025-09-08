use maud::{html, Markup, PreEscaped, DOCTYPE};
mod docs_sidebars;
mod header;

use docs_sidebars::{left_sidebar, right_sidebar};

pub use header::header;
use maudit::assets::StyleOptions;
use maudit::content::MarkdownHeading;
use maudit::maud::generator;
use maudit::page::{RenderResult, RouteContext};

pub fn docs_layout(
    main: Markup,
    ctx: &mut RouteContext,
    headings: &[MarkdownHeading],
) -> RenderResult {
    layout(
        html! {
            div.container.mx-auto."grid-cols-(--docs-columns)".grid."min-h-[calc(100%-64px)]" {
                aside.bg-linear-to-l."from-darker-white"."py-8"."h-full".border-r.border-r-borders {
                    (left_sidebar(ctx))
                }
                main.w-full.max-w-larger-prose.mx-auto.py-8 {
                    (main)
                }
                aside."py-8" {
                    (right_sidebar(headings))
                }
            }
        },
        true,
        false,
        ctx,
    )
}

pub fn layout(
    main: Markup,
    bottom_border: bool,
    licenses: bool,
    ctx: &mut RouteContext,
) -> RenderResult {
    ctx.assets
        .include_style_with_options("assets/prin.css", StyleOptions { tailwind: true });

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
                div.bg-our-white {
                    (header(ctx, bottom_border))
                    (main)
                    footer.bg-our-black.text-white {
                        div.container.mx-auto.px-8.py-8.flex.justify-between.items-center.flex-col-reverse."sm:flex-row".gap-y-12 {
                            div.grow."basis-[0]" {
                                a.text-md.font-bold href="https://bruits.org" {
                                    "Copyright © 2025 Bruits."
                                }
                                @if licenses {
                                    br;
                                    a.text-sm href="https://www.netlify.com" { "Site powered by Netlify" }
                                    p.text-sm {"Wax seal icon by " a href="https://game-icons.net/" { "Game-icons.net" } " under " a href="https://creativecommons.org/licenses/by/3.0/" { "CC BY 3.0" } }
                                }
                            }
                            div { (PreEscaped(include_str!("../assets/logo.svg")))}
                            div.flex.gap-x-6.grow.justify-end."basis-[0]".items-center {
                                a href="https://bsky.app/profile/bruits.org" {
                                    span.sr-only { "Follow Maudit on Bluesky" }
                                    (PreEscaped(include_str!("../assets/bsky.svg")))
                                }
                                a href="/chat/" {
                                    span.sr-only { "Join the Maudit community on Discord" }
                                    (PreEscaped(include_str!("../assets/discord.svg")))
                                }
                                a href="https://github.com/bruits/maudit" {
                                    span.sr-only { "View Maudit on GitHub" }
                                    (PreEscaped(include_str!("../assets/github.svg")))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    .into()
}
