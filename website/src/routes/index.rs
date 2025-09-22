use maud::html;
use maud::PreEscaped;
use maudit::route::prelude::*;

use crate::layout::layout;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
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
                    div."lg:container".mx-auto.relative {
                        div."px-4"."sm:py-8"."sm:mx-6"."sm:my-26"."my-14"."mb-20".flex.flex-col.justify-center.items-center.text-center {
                            h2."sm:text-6xl"."text-5xl"."sm:w-[22ch]"."xl:w-[30ch]"."mb-2"."leading-[1.15]" {
                                "Lo, " span.text-brand-red { "the still scrolls of the web"} ", unchanging and steadfast, at last!"
                            }
                            p.opacity-90.italic {
                                "Or, in simpler words, " span.text-brand-red {"a static site generator"} "."
                            }
                            div.mt-6.leading-tight {
                                a.btn.block.group.inline-block href="/docs/" { "Get Started" }
                                p.opacity-90.italic { "or scroll down to learn more" }
                            }
                        }
                    }
                }

                div."hero-background"."w-[175px]"."h-[175px]".absolute."left-1/2"."-translate-x-1/2"."-translate-y-1/2" {}

                div.h-14.bg-linear-to-b."from-darker-white".border-t.border-t-borders."sm:mb-24".mb-10 {}

                section.banner.py-14.text-center {
                    div."sm:px-52"."px-4"."lg:container".mx-auto {
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
            true,
            true,
            ctx,
        )
    }
}
