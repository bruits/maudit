use maud::html;
use maudit::page::prelude::*;

use crate::{
    content::ArticleContent,
    layout::layout,
    pages::{article::ArticleParams, Article},
};

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        let articles = ctx.content.get_source::<ArticleContent>("articles");
        let logo = ctx.assets.add_image("images/logo.svg");

        let markup = html! {
          (logo)
          ul {
            @for entry in &articles.entries {
              li {
                a href=(&Article.url(ArticleParams { article: entry.id.clone() })) {
                    h2 { (entry.data(ctx).title) }
                }
                p { (entry.data(ctx).description) }
              }
            }
          }
        }
        .into_string();

        layout(markup).into()
    }
}
