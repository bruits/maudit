use maud::html;
use maud::Markup;
use maud::PreEscaped;
use maudit::page::RouteContext;

pub fn header(_: &mut RouteContext, bottom_border: bool) -> Markup {
    let border = if bottom_border { "border-b" } else { "" };

    html! {
        header.px-8.py-4.text-our-black.bg-our-white."border-borders".(border) {
            div.container.flex.items-center.mx-auto.justify-between {
                div.flex.items-center.gap-x-8 {
                    a.flex.gap-x-2.items-center href="/" {
                        (PreEscaped(include_str!("../../assets/logo.svg")))
                        h1.text-2xl.tracking-wide { "Maudit" }
                    }
                    nav.text-lg.flex.gap-x-12.relative."top-[2px]" {
                        a href="/docs/" { "Documentation" }
                        a href="/news/" { "News" }
                        a href="/contribute/" { "Contribute" }
                        a href="https://github.com/bruits/maudit/issues/1" { "Roadmap" }
                    }
                }

                div.flex.gap-x-6 {
                    a href="/chat/" {
                        (PreEscaped(include_str!("../../assets/discord.svg")))
                    }
                    a href="https://github.com/bruits/maudit" {
                        (PreEscaped(include_str!("../../assets/github.svg")))
                    }
                }
            }
        }
    }
}
