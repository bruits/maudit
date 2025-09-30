use maud::{html, Markup, PreEscaped, DOCTYPE};
mod docs_sidebars;
mod header;

use docs_sidebars::{left_sidebar, right_sidebar};

pub use header::header;
use maudit::assets::StyleOptions;
use maudit::content::MarkdownHeading;
use maudit::maud::generator;
use maudit::route::{PageContext, RenderResult};

pub struct SeoMeta {
    pub title: String,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
}

impl Default for SeoMeta {
    fn default() -> Self {
        Self {
            title: "Maudit".to_string(),
            description: Some("A Rust library to build static websites.".to_string()),
            canonical_url: None,
        }
    }
}

impl SeoMeta {
    /// Create a new `SeoMeta` with the given title.
    pub fn render(&self, base_url: &Option<String>) -> Markup {
        let base_url = base_url.as_ref().unwrap();

        let formatted_title = if self.title == "Maudit" {
            self.title.clone()
        } else {
            format!("{} - Maudit", self.title)
        };

        let description = self
            .description
            .clone()
            .unwrap_or_else(|| SeoMeta::default().description.unwrap());

        let canonical_url = self.canonical_url.as_ref();

        let social_image_url = format!("{}/social-image.png", base_url);

        html! {
            title { (formatted_title) }
            meta name="description" content=(description);

            // Open Graph meta tags
            meta property="og:title" content=(formatted_title);
            meta property="og:description" content=(description);
            meta property="og:type" content="website";
            meta property="og:image" content=(social_image_url);
            @if let Some(canonical_url) = &canonical_url {
                meta property="og:url" content=(canonical_url);
                link rel="canonical" href=(canonical_url);
            }

            // Twitter Card meta tags
            meta name="twitter:card" content="summary";
            meta name="twitter:title" content=(formatted_title);
            meta name="twitter:description" content=(description);
            meta name="twitter:image" content=(social_image_url);
        }
    }
}

pub fn docs_layout(
    main: Markup,
    ctx: &mut PageContext,
    headings: &[MarkdownHeading],
    seo: Option<SeoMeta>,
) -> impl Into<RenderResult> {
    ctx.assets.include_script("assets/docs-sidebar.ts");

    layout(
        html! {
            // Second header for docs navigation (mobile only)
            header.bg-our-white.border-b.border-borders.md:hidden.bg-linear-to-b."from-darker-white" {
                div.flex.items-center.justify-between {
                    button id="left-sidebar-toggle" .px-4.py-3.flex.items-center.gap-x-2.text-base.font-medium.text-our-black aria-label="Toggle navigation menu" {
                        (PreEscaped(include_str!("../assets/side-menu.svg")))
                        span { "Menu" }
                    }
                    button id="right-sidebar-toggle" .px-4.py-3.flex.items-center.gap-x-2.text-base.font-medium.text-our-black aria-label="Toggle table of contents" {
                        span { "On this page" }
                        (PreEscaped(include_str!("../assets/toc.svg")))
                    }
                }
            }

            // Mobile left sidebar overlay
            div id="mobile-left-sidebar" .fixed.left-0.w-full.bg-our-white.transform."-translate-x-full".transition-all.opacity-0.pointer-events-none.z-50.overflow-y-auto style="top: 116px; bottom: 0;" {
                div.px-6.py-4 {
                    (left_sidebar(ctx))
                }
            }

            // Mobile right sidebar overlay
            div id="mobile-right-sidebar" .fixed.right-0.w-full.bg-our-white.transform."translate-x-full".transition-all.opacity-0.pointer-events-none.z-50.overflow-y-auto style="top: 116px; bottom: 0;" {
                div.px-6.py-4 {
                    (right_sidebar(headings))
                }
            }

            div.container.mx-auto."md:grid-cols-(--docs-columns)".md:grid."min-h-[calc(100%-64px)]".px-4.md:px-0.pt-2.md:pt-0 {
                aside.bg-linear-to-l."from-darker-white"."py-8"."h-full".border-r.border-r-borders.hidden.md:block {
                    (left_sidebar(ctx))
                }
                main.w-full.max-w-larger-prose.mx-auto.md:pt-8.py-4.pb-8.md:pb-16 {
                    (main)
                }
                aside."py-8".hidden."md:block" {
                    (right_sidebar(headings))
                }
            }
        },
        true,
        false,
        ctx,
        seo,
    )
}

pub fn layout(
    main: Markup,
    bottom_border: bool,
    licenses: bool,
    ctx: &mut PageContext,
    seo: Option<SeoMeta>,
) -> impl Into<RenderResult> {
    ctx.assets
        .include_style_with_options("assets/prin.css", StyleOptions { tailwind: true });

    let seo_data = seo.unwrap_or_default();

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                (generator())
                link rel="icon" href="/favicon.svg";
                (seo_data.render(&ctx.base_url))
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
}
