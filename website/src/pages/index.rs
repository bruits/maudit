use maud::html;
use maud::PreEscaped;
use maudit::page::prelude::*;

use crate::layout::layout;

#[route("/")]
pub struct Index;

const LATEST_NEWS: (&str, &str) = ("Maudit v0.1.0 is out", "/");

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let features = [
            ("Performant", "Generate a site with thousands of pages in seconds using minimal resources."),
            ("Content", "Bring your content to life with built-in support for Markdown, syntax highlighting, and more."),
            ("SEO-optimized", "Ensure your site is SEO-friendly by default with built-in support for sitemaps."),
            ("Powerful routing", "Flexible and powerful routing system allows you to create complex sites with ease."),
            ("Ecosystem-ready", "Maudit utilize <a class=\"underline\" href=\"https://rolldown.rs\">Rolldown</a>, a fast bundler for JavaScript and CSS, enabling the usage of TypeScript and the npm ecosystem."),
            ("Bring your templates", "Use your preferred templating engine to craft your website's pages. If it renders to HTML, Maudit supports it."),
            ("Type-safe routing", "Ensure your links stay correct, even through site refactors."),
            ("Style your way", "Supports popular CSS frameworks and preprocessors, like Tailwind CSS and Sass.")
        ].map(|(name, description)| {(name, PreEscaped(description))});

        layout(
            html! {
                div.w-full {
                    div."lg:container".mx-auto.hero-background {
                        div.px-6.py-10.mx-6.my-20 {
                            a.bg-our-black.text-our-white.rounded-sm.px-2.py-1.text-sm.mb-2.inline-block."hover:text-white"."hover:cursor-pointer" href=(LATEST_NEWS.1) {
                                (format!("{} →", LATEST_NEWS.0))
                            }
                            h2.text-5xl."w-[22ch]"."xl:w-[30ch]"."mb-1"."leading-[1.15]" {
                                "Lo, " span.text-brand-red { "the still scrolls of the web"} ", unchanging and steadfast, at last!"
                            }
                            p.opacity-75.italic {
                                "Or, in simpler words, " span.text-brand-red {"a static site generator"} "."
                            }
                            div.mt-6.leading-tight {
                                a.btn.block.group.inline-block href="/docs/" { "Get Started" span.inline-block."group-hover:translate-x-3".transition-transform.translate-x-2 { "→" } }
                                p.opacity-75.italic { "or scroll down to learn more" }
                            }
                        }
                    }
                }
                div.h-12.bg-linear-to-b."from-darker-white".border-t.border-t-borders{}

                div."px-12"."lg:container".mx-auto.mb-12 {
                    section {
                        h3.text-3xl."mb-4".border-b.border-b-borders.inline-block { "1. Capacities." }
                        div.grid."grid-cols-1"."lg:grid-cols-3"."gap-y-4"."lg:gap-x-6 lg:gap-y-6" {
                            @for (name, description) in features {
                                div.card {
                                    h4.text-xl.font-bold { (name) }
                                    p { (description) }
                                }
                            }
                            span.opacity-75.italic.block.mt-1 { "And a lot more!" }
                        }
                    }
                }

                section.banner.mb-4.py-12 {
                    div."px-12"."lg:container".mx-auto {
                        h3.text-3xl."mb-4".border-b.border-borders.inline-block { "2. Crafted for timeless sites." }
                        p.font-bold {
                            "Maudit was built for one purpose: creating static websites."
                        }
                        p {
                            "By focusing only on static sites, Maudit is able to provide every optimization and features needed for fast performance, low maintenance, and effortless reliability—making it a better fit than general-purpose tools. "
                            a.font-bold."hover:text-our-black" href="/docs/philosophy" { "Read our philosophy." }
                        }
                    }
                }

            },
            false,
            ctx,
        )
    }
}
