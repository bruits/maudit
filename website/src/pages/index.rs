use maud::html;
use maud::PreEscaped;
use maudit::page::prelude::*;

use crate::layout::layout;

#[route("/")]
pub struct Index;

const LATEST_NEWS: (&str, &str) = ("Introducing Maudit", "/news/maudit01");

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let features = [
            ("Performant", "Generate a site with thousands of pages in seconds using minimal resources."),
            ("Content", "Bring your content to life with built-in support for Markdown, custom components, syntax highlighting, and more."),
            ("Style your way", "Style with plain CSS, or opt for frameworks and preprocessors such as Tailwind and Sass."),
            ("Powerful routing", "Flexible and powerful routing system allows you to create complex sites with ease."),
            ("Ecosystem-ready", "Maudit utilize <a class=\"underline\" href=\"https://rolldown.rs\">Rolldown</a>, a fast bundler for JavaScript and CSS, enabling the usage of TypeScript and the npm ecosystem."),
            ("Bring your templates", "Use your preferred templating engine to craft your website's pages. If it renders to HTML, Maudit supports it."),
        ].map(|(name, description)| {(name, PreEscaped(description))});

        layout(
            html! {
                div.w-full {
                    div."lg:container".mx-auto.hero-background.relative {
                        div.px-4.py-14.mx-6.my-28 {
                            a.bg-our-black.text-our-white.rounded-sm.px-2.py-1.text-sm.mb-2.inline-block."hover:!text-brighter-brand"."hover:cursor-pointer" href=(LATEST_NEWS.1) {
                                (format!("{}", LATEST_NEWS.0))
                            }
                            h2.text-5xl."w-[22ch]"."xl:w-[30ch]"."mb-1"."leading-[1.15]" {
                                "Lo, " span.text-brand-red { "the still scrolls of the web"} ", unchanging and steadfast, at last!"
                            }
                            p.opacity-80.italic {
                                "Or, in simpler words, " span.text-brand-red {"a static site generator"} "."
                            }
                            div.mt-6.leading-tight {
                                a.btn.block.group.inline-block href="/docs/" { "Get Started" span.inline-block."group-hover:translate-x-3".transition-transform.translate-x-2 { "→" } }
                                p.opacity-80.italic { "or scroll down to learn more" }
                            }
                        }
                    }
                }

                div.h-14.bg-linear-to-b."from-darker-white".border-t.border-t-borders{}

                section.banner.py-14.text-center {
                    div."px-52"."lg:container".mx-auto {
                        h3.text-4xl."mb-5".inline-block { "Crafted for timeless sites" }
                        p.font-bold {
                            "Maudit was built for one purpose: creating static websites."
                        }
                        p {
                            "This devotion to static sites ensures speed, simple upkeep, and effortless reliability."
                            div."mt-4" {
                                a.font-bold.text-lg."hover:!text-our-black" href="/docs/philosophy" { "Read our philosophy" }
                            }
                        }
                    }
                }

                section.features.py-14 {
                    div."px-12"."lg:container".mx-auto {
                        div.grid."grid-cols-1"."md:grid-cols-2"."lg:grid-cols-3"."gap-8"."gap-y-12" {
                            @for (name, description) in features {
                                div.feature-card {
                                    h3.text-2xl.font-bold.mb-2 { (name) }
                                    p { (description) }
                                }
                            }
                        }
                    }
                }

                div.h-12.bg-linear-to-b."from-darker-white".border-t.border-t-borders{}

            },
            false,
            true,
            ctx,
        )
    }
}
