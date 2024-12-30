use maud::html;
use maudit::page::prelude::*;

use crate::layout::layout;

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
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
                                a.btn.block.group.inline-block href="/docs" { "Get Started" span.inline-block."group-hover:translate-x-3".transition-transform.translate-x-2 { "→" } }
                                p.opacity-75.italic { "or scroll down to learn more" }
                            }
                        }
                    }
                }
                div.h-12.bg-gradient-to-b."from-darker-white".border-t.border-t-borders{}

                div."px-12"."lg:container".mx-auto {
                    section {
                        h3.text-3xl."mb-4" { "1. Features" }
                        p { "TODO: Find a subtitle" }
                    }
                }

            },
            false,
            ctx,
        )
    }
}
