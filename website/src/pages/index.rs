use maud::html;
use maudit::page::prelude::*;

use crate::layout::{header, layout};

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        layout(
            html! {
                (header(ctx))
                div.w-screen {
                    div."lg:container".mx-auto.hero-background {
                        div.px-6.py-10.mx-6.my-14 {
                            h2.text-5xl."w-[24ch]"."xl:w-[30ch]"."mb-1"."leading-[1.15]" { "Lo, " span.text-brand-red { "the still scrolls of the web"} ", unchanging and steadfast, at last!" }
                            p.opacity-75.italic { "Or, in simpler words, " span.text-brand-red {"a static site generator"} "." }
                            div.mt-6.leading-tight {
                                a.btn.block.group href="/docs" { "Get Started" span.inline-block."group-hover:translate-x-4".transition-transform.translate-x-2 { "→" } }
                                span.opacity-75.italic { "or scroll down to learn more" }
                            }
                        }
                    }
                }
                div.h-12.bg-our-black.bg-opacity-5 {}

            },
            ctx,
        )
    }
}
