//! Blog archetype.
//! Represents a markdown blog archetype, with an index page and individual entry pages.
use crate::layouts::layout;
use maud::{html, Markup};
use maudit::content::markdown_entry;
use maudit::page::prelude::*;
use maudit::page::FullRoute;

pub fn blog_index_content<T: FullRoute>(
    route: impl FullRoute,
    ctx: &mut PageContext,
    name: &str,
    stringified_ident: &str,
) -> Markup {
    let blog_entries = ctx
        .content
        .get_source::<BlogEntryContent>(stringified_ident);

    let markup = html! {
        main {
            @for entry in &blog_entries.entries {
                a href=(route.url(&BlogEntryParams { entry: entry.id.clone() }.into())) {
                    h2 { (entry.data(ctx).title) }
                    p { (entry.data(ctx).description) }
                }
            }
        }
    }
    .into_string();

    layout(name, markup)
}

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct BlogEntryContent {
    pub title: String,
    pub description: String,
}

#[derive(Params, Clone)]
pub struct BlogEntryParams {
    pub entry: String,
}

pub fn blog_entry_routes(ctx: &mut DynamicRouteContext, name: &str) -> Pages<BlogEntryParams> {
    let blog_entries = ctx.content.get_source::<BlogEntryContent>(name);

    blog_entries.into_pages(|entry| {
        Page::from_params(BlogEntryParams {
            entry: entry.id.clone(),
        })
    })
}

pub fn blog_entry_render(ctx: &mut PageContext, name: &str, stringified_ident: &str) -> Markup {
    let params = ctx.params::<BlogEntryParams>();
    let blog_entries = ctx
        .content
        .get_source::<BlogEntryContent>(stringified_ident);
    let blog_entry = blog_entries.get_entry(&params.entry);

    let headings = blog_entry.data(ctx).get_headings();
    println!("{:?}", headings);

    layout(name, blog_entry.render(ctx))
}
