use maud::{html, Markup};
use maudit::page::RouteContext;

use crate::content::DocsContent;

pub fn sidebar(ctx: &mut RouteContext) -> Markup {
    let content = ctx.content.get_source::<DocsContent>("docs");

    // Map entries into HTML
    let mut sections = std::collections::HashMap::new();

    for entry in content.entries.iter() {
        if let Some(section) = &entry.data.section {
            sections.entry(section).or_insert_with(Vec::new).push(entry);
        }
    }

    let entries = sections.iter().map(|(section, entries)| {
        html! {
            li {
                h2.text-lg.font-bold { (section) }
                ul.pl-1 {
                    @for entry in entries {
                        li {
                            a href=(format!("/docs/{}", entry.id)) { (entry.data.title) }
                        }
                    }
                }
            }
        }
    });

    html! {
        ul {
            @for entry in entries {
                (entry)
            }
        }
    }
}
