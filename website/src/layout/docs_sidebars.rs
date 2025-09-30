use maud::{Markup, html};
use maudit::{
    content::MarkdownHeading,
    route::{PageContext, RouteExt},
};

use crate::{
    content::{DocsContent, DocsSection},
    routes::{DocsPage, DocsPageParams},
};

pub fn left_sidebar(ctx: &mut PageContext) -> Markup {
    let content = ctx.content.get_source::<DocsContent>("docs");

    let mut sections = std::collections::HashMap::new();

    for entry in content.entries.iter() {
        if let Some(section) = &entry.data(ctx).section {
            sections.entry(section).or_insert_with(Vec::new).push(entry);
        }
    }

    let mut sections: Vec<_> = sections.into_iter().collect();

    // TODO: Implement sorting on the enum ord itself?
    sections.sort_by_key(|(section, _)| {
        // Define sort order
        match section {
            DocsSection::GettingStarted => 0,
            DocsSection::CoreConcepts => 1,
            DocsSection::Guide => 2,
            DocsSection::Advanced => 3,
        }
    });

    let static_links: Vec<(&str, &str)> = vec![
        ("Reference", "https://docs.rs/maudit"),
        (
            "Examples",
            "https://github.com/bruits/maudit/tree/main/examples",
        ),
    ];

    let entries = sections.iter().map(|(section, entries)| {
        html! {
            li.mb-6.sm:mb-4 {
                h2.text-xl.sm:text-lg.font-bold { (section) }
                ul {
                    @for entry in entries {
                        @let url = DocsPage.url(DocsPageParams { slug: entry.id.clone() });
                        @let is_current_page = url == *ctx.current_path;
                        li {
                            @let base_classes = "block py-2 sm:py-1 px-4 sm:px-3 text-lg sm:text-base font-medium sm:font-normal transition-colors border-b border-borders sm:border-0";
                            @let conditional_classes = if is_current_page {
                                "text-brand-red sm:border-l-2 sm:border-l-brand-red"
                            } else {
                                "text-our-black hover:text-brand-red sm:border-l-2 sm:border-l-borders sm:hover:border-l-brand-red"
                            };
                            a class=(format!("{} {}", base_classes, conditional_classes)) href=(url) {
                                (entry.data(ctx).title)
                            }
                        }
                    }
                }
            }
        }
    });

    html! {
        ul.mb-6.sm:mb-4.space-y-0.sm:space-y-1 {
            @for (name, link) in static_links {
                li {
                    a.block.py-2.sm:py-0.px-4.sm:px-0.text-xl.sm:text-lg.font-medium.text-our-black.border-b.border-borders.sm:border-0.transition-colors."hover:text-brand-red".sm:bg-transparent.sm:hover:bg-transparent href=(link) { (name) }
                }
            }
        }
        ul.space-y-1 {
            @for entry in entries {
                (entry)
            }
        }
    }
}

pub fn right_sidebar(headings: &[MarkdownHeading]) -> Markup {
    let mut html_headings: Vec<maud::PreEscaped<String>> = Vec::new();
    let mut i = 0;
    let mut seen_h2 = false;
    while i < headings.len() {
        let heading = &headings[i];
        let (pad, border) = match heading.level {
            2 => ("pl-0", ""),                                 // h2
            3 => ("pl-4", "sm:border-l-2 sm:border-borders"),  // h3
            4 => ("pl-8", "sm:border-l-2 sm:border-borders"),  // h4
            5 => ("pl-12", "sm:border-l-2 sm:border-borders"), // h5
            6 => ("pl-16", "sm:border-l-2 sm:border-borders"), // h6
            _ => ("pl-0", ""),                                 // fallback
        };
        let next_level = if i + 1 < headings.len() {
            headings[i + 1].level
        } else {
            0
        };
        let margin_top = if heading.level == 2 && next_level > 2 && seen_h2 {
            "mt-4"
        } else {
            ""
        };
        if heading.level == 2 {
            seen_h2 = true;
        }
        html_headings.push(html! {
            li.(border).(margin_top) {
                a class=(format!("block py-1 px-3 sm:py-0 text-lg sm:text-base transition-colors hover:bg-gray-50 sm:hover:bg-transparent hover:text-brand-red border-b border-borders sm:border-b-0 {}", pad)) href=(format!("#{}", heading.id)) {
                    (heading.title)
                }
            }
        });
        i += 1;
    }

    html!(
        h2.text-xl.sm:text-lg.font-bold.mb-4.sm:mb-0 { "On This Page" }
        nav.sticky.top-8 {
            ul {
                @for heading in html_headings {
                    (heading)
                }
            }
        }
    )
}
