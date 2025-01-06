use maud::{html, Markup, PreEscaped};
use maudit::{content::ContentEntry, page::prelude::*};

use crate::{content::DocsContent, layout::docs_layout};

#[route("/docs")]
pub struct DocsIndex;

impl Page for DocsIndex {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let index_page = ctx
            .content
            .get_source::<DocsContent>("docs")
            .get_entry("index");

        docs_layout(render_entry(index_page), ctx)
    }
}

fn render_entry(entry: &ContentEntry<DocsContent>) -> Markup {
    html! {
        section.mb-4.border-b."border-[#e9e9e7]".pb-2 {
            @if let Some(section) = &entry.data.section {
                p.text-sm.font-bold.mb-2 { (section) }
            }
            h2.text-5xl.font-bold.mb-2 { (entry.data.title) }
            @if let Some(description) = &entry.data.description {
                h3.text-lg { (description) }
            }
        }
        section.prose."lg:prose-lg".max-w-none {
            (PreEscaped((entry.render)()))
        }
    }
}

#[route("/docs/[slug]")]
pub struct DocsPage;

#[derive(Params)]
struct DocsPageParams {
    slug: String,
}

impl DynamicRoute<DocsPageParams> for DocsPage {
    fn routes(&self, ctx: &DynamicRouteContext) -> Vec<DocsPageParams> {
        let content = ctx.content.get_source::<DocsContent>("docs");

        content.into_params(|entry| DocsPageParams {
            slug: entry.id.clone(),
        })
    }
}

impl Page for DocsPage {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let entry = ctx
            .content
            .get_source::<DocsContent>("docs")
            .get_entry(&ctx.params::<DocsPageParams>().slug);

        docs_layout(render_entry(entry), ctx)
    }
}
