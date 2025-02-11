//! Blog archetype.
//! Represents a markdown blog archetype, with an index page and individual entry pages.
use crate::layouts::layout;
use maud::{html, Markup};
use maudit::content::markdown_entry;
use maudit::page::prelude::*;

pub fn blog_index_content<T: FullPage>(ctx: &mut RouteContext, name: &str) -> Markup {
    let blog_entries = ctx.content.get_source::<BlogEntryContent>(name);

    let markup = html! {
      ul {
        @for entry in &blog_entries.entries {
          li {
            a href=(T::url_unsafe(BlogEntryParams { entry: entry.id.clone() })) {
                h2 { (entry.data.title) }
            }
            p { (entry.data.description) }
          }
        }
      }
    }
    .into_string();

    layout(markup)
}

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct BlogEntryContent {
    pub title: String,
    pub description: String,
}

#[derive(Params)]
pub struct BlogEntryParams {
    pub entry: String,
}

pub fn blog_entry_routes(ctx: &mut DynamicRouteContext, name: &str) -> Vec<BlogEntryParams> {
    let blog_entries = ctx.content.get_source::<BlogEntryContent>(name);

    blog_entries.into_params(|entry| BlogEntryParams {
        entry: entry.id.clone(),
    })
}

pub fn blog_entry_render(ctx: &mut RouteContext, name: &str) -> Markup {
    let params = ctx.params::<BlogEntryParams>();
    let blog_entries = ctx.content.get_source::<BlogEntryContent>(name);
    let blog_entry = blog_entries.get_entry(&params.entry);

    let headings = blog_entry.data.get_headings();
    println!("{:?}", headings);

    layout(blog_entry.render())
}
