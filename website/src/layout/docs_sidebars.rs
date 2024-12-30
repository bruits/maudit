use maud::{html, Markup};
use maudit::page::RouteContext;

use crate::content::{DocsContent, DocsSection};

pub fn left_sidebar(ctx: &mut RouteContext) -> Markup {
    let content = ctx.content.get_source::<DocsContent>("docs");

    let mut sections = std::collections::HashMap::new();

    for entry in content.entries.iter() {
        if let Some(section) = &entry.data.section {
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
        }
    });

    let static_links: Vec<(&str, &str)> = vec![
        ("Reference", "https://docs.rs/maudit"),
        (
            "Examples",
            "https://github.com/Princesseuh/maudit/tree/main/examples",
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
                        li."border-l-2"."border-borders"."hover:border-brand-red"."pl-3"."py-1".(if is_current_page { "text-brand-red border-brand-red" } else { "" }) {
                            a.block href=(format!("/docs/{}/", entry.id)) { (entry.data.title) } // TODO: Use type-safe routing
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

pub fn right_sidebar(_: &mut RouteContext) -> Markup {
    html!(
        h2.text-lg.font-bold { "On This Page" }
        nav.sticky.top-8 {
            // TODO: Implement this properly
            ul {
                li {
                    a href="#hello-world" { "Hello World" }
                }
            }
        }
    )
}
