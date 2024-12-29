use maud::html;
use maud::Markup;
use maud::PreEscaped;
use maudit::page::RouteContext;

pub fn header(_: &mut RouteContext) -> Markup {
    html! {
        header.px-8.py-4.bg-faded-black.text-our-white {
            div.container.flex.items-center.gap-x-8 {
                a.flex.gap-x-2.items-center."hover:text-brighter-brand" href="/" {
                    (PreEscaped(include_str!("../../assets/logo.svg")))
                    h1.text-2xl.tracking-wide { "Maudit" }
                }
                nav.text-lg.flex.gap-x-12.relative."top-[2px]" {
                    a."hover:text-brighter-brand" href="/docs" { "Documentation" }
                    a."hover:text-brighter-brand" href="/news" { "News" }
                    a."hover:text-brighter-brand" href="/community" { "Community" }
                }
                div {}
            }
        }
    }
}
