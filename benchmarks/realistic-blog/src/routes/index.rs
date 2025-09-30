use maud::html;
use maudit::route::prelude::*;

use crate::{
    content::ArticleContent,
    layout::layout,
    routes::{
        Article, Articles,
        article::{ArticleParams, ArticlesParams},
    },
};

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let mut articles = ctx
            .content
            .get_source::<ArticleContent>("articles")
            .entries
            .iter()
            .collect::<Vec<_>>(); // Collect into a Vec to allow sorting

        // Sort by date, newest first
        articles.sort_by(|a, b| b.data(ctx).date.cmp(&a.data(ctx).date));

        // Take three latest
        articles = articles.into_iter().take(3).collect::<Vec<_>>();

        let markup = html! {
          h2 { "Hello!" }
            p { "Welcome to my blog. I'm a super real blog that was totally not created to serve as a benchmark. In my articles, you'll find various content such as, for example, 36 guides on how to use Markdown. Suspiciously, some of them are slightly different." }

          h2 { "Latest Articles" }
          ul.articles-list {
            @for entry in &articles {
              li {
                a href=(&Article.url(ArticleParams { article: entry.id.clone() })) {
                    h2 { (entry.data(ctx).title) }
                }
                p { (entry.data(ctx).description) }
                span { (entry.data(ctx).date) }
              }
            }
          }
          a href=(&Articles.url(ArticlesParams { page: None })) { "See all articles..." }
        }
        .into_string();

        layout(ctx, markup)
    }
}
