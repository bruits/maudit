use maud::html;
use maud::Markup;
use maud::PreEscaped;
use maudit::route::PageContext;

pub fn header(ctx: &mut PageContext, bottom_border: bool) -> Markup {
    ctx.assets.include_script("assets/mobile-menu.ts");

    let border = if bottom_border { "border-b" } else { "" };
    let nav_links = vec![
        ("/docs/", "Documentation"),
        ("/news/", "News"),
        ("/contribute/", "Contribute"),
        ("https://github.com/bruits/maudit/issues/1", "Roadmap"),
    ];
    let social_links = vec![
        (
            "/chat/",
            "Join our Discord",
            include_str!("../../assets/discord.svg"),
        ),
        (
            "https://github.com/bruits/maudit",
            "View on GitHub",
            include_str!("../../assets/github.svg"),
        ),
    ];

    html! {
        header.px-4.md:px-8.py-4.text-our-black.bg-our-white."border-borders".(border) {
            div.container.flex.items-center.mx-auto.justify-between {
                div.flex.items-center.gap-x-8 {
                    a.flex.gap-x-2.items-center href="/" {
                        (PreEscaped(include_str!("../../assets/logo.svg")))
                        h1.text-2xl.tracking-wide { "Maudit" }
                    }
                    nav.text-lg.gap-x-12.relative."top-[2px]".hidden."md:flex" {
                        @for (href, text) in &nav_links {
                            a href=(href) { (text) }
                        }
                    }
                }

                div.gap-x-6.hidden.md:flex {
                    @for (href, _text, icon_svg) in &social_links {
                        a href=(href) {
                            span.sr-only { (_text) }
                            (PreEscaped(icon_svg))
                        }
                    }
                }

                div.md:hidden.flex.align-middle.justify-center.items-center {
                    button id="mobile-menu-button" aria-label="Toggle main menu" {
                        span id="hamburger-icon" {
                            (PreEscaped(include_str!("../../assets/hamburger.svg")))
                        }
                        span id="close-icon" .hidden {
                            (PreEscaped(include_str!("../../assets/close.svg")))
                        }
                    }
                }
            }
        }

        // Mobile menu panel
        div id="mobile-menu-panel" .fixed.left-0.w-full.bg-our-white.transform."-translate-x-4".transition-all.opacity-0.pointer-events-none.z-50 style="top: 65px; bottom: 0;" {
            nav {
                @for (href, text) in &nav_links {
                    a.block.text-2xl.font-medium.text-our-black.px-4.py-4.border-b.border-borders href=(href) { (text) }
                }
            }
            div.px-6.py-8.flex.flex-wrap.gap-8 {
                @for (href, text, icon_svg) in &social_links {
                    a.flex.items-center href=(href) {
                        span.sr-only { (text) }
                        (PreEscaped(icon_svg))
                    }
                }
            }
        }
    }
}
