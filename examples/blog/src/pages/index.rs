use maud::html;
use maudit::page::prelude::*;

use crate::{
    content::ArticleContent,
    layout::layout,
    pages::{article::ArticleParams, Article},
};

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let articles = ctx.content.get_collection::<ArticleContent>("articles");

        let markup = html! {
          ul {
            @for entry in &articles.entries {
              li {
                a href=(Article::url_unsafe(ArticleParams { article: entry.id.clone() })) {
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
