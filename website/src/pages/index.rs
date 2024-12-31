use maud::html;
use maudit::page::prelude::*;

use crate::layout::layout;

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let features = [("Fast", "Maudit is fast, and it's not just because it's a static site generator. It's also because it's written in Rust."),
            ("Flexible", "Maudit is designed to be flexible and extensible. It's easy to add new features and customize the output."),
            ("Type-safe", "Maudit uses type-safe routing, so you can't link to a page that doesn't exist. It also has a built-in search feature."),
            ("Easy to use", "Maudit is easy to use, even if you're not familiar with Rust. It has a simple API and clear documentation.")];

        layout(
            html! {
                div.w-screen {
                    div."lg:container".mx-auto.hero-background {
                        div.px-6.py-10.mx-6.my-14 {
                            a.bg-our-black.text-our-white.rounded-sm.px-2.py-1.text-sm.mb-2.inline-block."hover:text-white"."hover:cursor-pointer" href="#" {
                                "Maudit v0.1.0 is out →"
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
                div.h-12.bg-gradient-to-b."from-darker-white".border-t.border-t-borders{}

                div."px-12"."lg:container".mx-auto.mb-12 {
                    section {
                        h3.text-3xl."mb-4".border-b.border-b-borders.inline-block { "1. Features." }
                        div.grid."grid-cols-1"."lg:grid-cols-2"."gap-y-4"."lg:gap-x-8" {
                            @for (name, description) in features {
                                div.card {
                                    h4.text-xl.font-bold { (name) }
                                    p { (description) }
                                }
                            }
                        }
                    }
                }

                section.banner.mb-4.py-12 {
                    div."px-12"."lg:container".mx-auto {
                        h3.text-3xl."mb-4".border-b.border-borders.inline-block { "2. Built for static websites." }
                        p.font-bold {
                            "Maudit was built for one purpose: creating static websites."
                        }
                        p {
                            "By focusing only on static sites, Maudit is able to provide every optimization and features needed for fast performance, low maintenance, and effortless reliability—making it a better fit than general-purpose tools."
                        }
                    }
                }

            },
            false,
            ctx,
        )
    }
}
