use maud::{html, Markup, PreEscaped};
use maudit::{content::ContentEntry, route::prelude::*};

use crate::{content::DocsContent, layout::docs_layout};

#[route("/docs")]
pub struct DocsIndex;

impl Route for DocsIndex {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let index_page = ctx
            .content
            .get_source::<DocsContent>("docs")
            .get_entry("index");

        let headings = index_page.data(ctx).get_headings().clone();

        docs_layout(render_entry(index_page, ctx), ctx, &headings)
    }
}

fn render_entry(entry: &ContentEntry<DocsContent>, ctx: &mut PageContext) -> Markup {
    html! {
        section.mb-4.border-b."border-[#e9e9e7]".pb-2 {
            @if let Some(section) = &entry.data(ctx).section {
                p.text-sm.font-bold { (section) }
            }
            h2.text-5xl.font-bold.mb-2 { (entry.data(ctx).title) }
            @if let Some(description) = &entry.data(ctx).description {
                h3.text-lg { (description) }
            }
        }
        section.prose."lg:prose-lg".max-w-none {
            (PreEscaped(entry.render(ctx)))
        }
    }
}

#[route("/docs/[slug]")]
pub struct DocsPage;

#[derive(Params, Clone)]
struct DocsPageParams {
    slug: String,
}

impl Route<DocsPageParams> for DocsPage {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<DocsPageParams> {
        let content = ctx.content.get_source::<DocsContent>("docs");

        content.into_pages(|entry| {
            Page::from_params(DocsPageParams {
                slug: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let slug = ctx.params::<DocsPageParams>().slug.clone();
        let entry = ctx
            .content
            .get_source::<DocsContent>("docs")
            .get_entry(&slug);

        let headings = entry.data(ctx).get_headings().clone();
        docs_layout(render_entry(entry, ctx), ctx, &headings)
    }
}
