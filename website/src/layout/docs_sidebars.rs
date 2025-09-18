use maud::{html, Markup};
use maudit::{content::MarkdownHeading, page::RouteContext};

use crate::content::{DocsContent, DocsSection};

pub fn left_sidebar(ctx: &mut RouteContext) -> Markup {
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
            li.mb-4 {
                h2.text-lg.font-bold { (section) }
                ul {
                    @for entry in entries {
                        @let url = format!("/docs/{}", entry.id);
                        @let is_current_page = url == ctx.current_url;
                        li."border-l-2"."hover:border-brand-red"."pl-3"."py-1".(if is_current_page { "text-brand-red border-brand-red" } else { "border-borders" }) {
                            a.block href=(format!("/docs/{}/", entry.id)) { (entry.data(ctx).title) } // TODO: Use type-safe routing
                        }
                    }
                }
            }
        }
    });

    html! {
        ul.mb-4 {
            @for (name, link) in static_links {
                li.mb-1 {
                    a.text-lg href=(link) { (name) }
                }
            }
        }
        ul {
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
            2 => ("pl-0", ""),                           // h2
            3 => ("pl-4", "border-l-2 border-borders"),  // h3
            4 => ("pl-8", "border-l-2 border-borders"),  // h4
            5 => ("pl-12", "border-l-2 border-borders"), // h5
            6 => ("pl-16", "border-l-2 border-borders"), // h6
            _ => ("pl-0", ""),                           // fallback
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
            li.(pad).(border).(margin_top) {
                a href=(format!("#{}", heading.id)) { (heading.title) }
            }
        });
        i += 1;
    }

    html!(
        h2.text-lg.font-bold { "On This Page" }
        nav.sticky.top-8 {
            ul {
                @for heading in html_headings {
                    (heading)
                }
            }
        }
    )
}
