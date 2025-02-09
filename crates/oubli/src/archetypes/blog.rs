//! Blog archetype.
//! Represents a markdown blog archetype, with an index page and individual entry pages.
use crate::layouts::layout;
use maud::{html, Markup};
use maudit::content::markdown_entry;
use maudit::page::prelude::*;

use super::NimporteQuoi;

#[route("/blog")]
pub struct BlogIndex;

impl Page for BlogIndex {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let blog_entries = ctx.content.get_source::<BlogEntryContent>("blog_entry");

        let markup = html! {
          ul {
            @for entry in &blog_entries.entries {
              li {
                a href=(BlogEntry::url_unsafe(BlogEntryParams { entry: entry.id.clone() })) {
                    h2 { (entry.data.title) }
                }
                p { (entry.data.description) }
              }
            }
          }
        }
        .into_string();

        layout(markup).into()
    }
}

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct BlogEntryContent {
    pub title: String,
    pub description: String,
}

impl NimporteQuoi for BlogEntryContent {}

#[route("/blog/[entry]")]
pub struct BlogEntry;

#[derive(Params)]
pub struct BlogEntryParams {
    pub entry: String,
}

impl DynamicRoute<BlogEntryParams> for BlogEntry {
    fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<BlogEntryParams> {
        let blog_entries = ctx.content.get_source::<BlogEntryContent>("blog_entry");

        blog_entries.into_params(|entry| BlogEntryParams {
            entry: entry.id.clone(),
        })
    }
}

impl Page<Markup> for BlogEntry {
    fn render(&self, ctx: &mut RouteContext) -> Markup {
        let params = ctx.params::<BlogEntryParams>();
        let blog_entries = ctx.content.get_source::<BlogEntryContent>("blog_entry");
        let blog_entry = blog_entries.get_entry(&params.entry);

        let headings = blog_entry.data.get_headings();
        println!("{:?}", headings);

        layout(blog_entry.render())
    }
}
