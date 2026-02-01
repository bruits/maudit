use maud::html;
use maudit::route::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::content::ArticleContent;

/// Dynamic route for individual articles - uses `get_entry()` which tracks only the accessed file
#[route("/articles/[slug]")]
pub struct Article;

#[derive(Params, Clone)]
pub struct ArticleParams {
    slug: String,
}

impl Route<ArticleParams> for Article {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<ArticleParams> {
        let articles = ctx.content.get_source::<ArticleContent>("articles");

        // into_pages tracks all files (for generating the list of pages)
        articles.into_pages(|entry| {
            Page::from_params(ArticleParams {
                slug: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let slug = ctx.params::<ArticleParams>().slug.clone();
        let articles = ctx.content.get_source::<ArticleContent>("articles");

        // get_entry tracks only THIS specific file
        let article = articles.get_entry(&slug);

        // Generate a unique build ID - uses nanoseconds for uniqueness
        let build_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "0".to_string());

        html! {
            html {
                head {
                    title { (article.data(ctx).title) }
                }
                body data-build-id=(build_id) {
                    h1 id="title" { (article.data(ctx).title) }
                    p id="description" { (article.data(ctx).description) }
                    div id="content" {
                        (maud::PreEscaped(article.render(ctx)))
                    }
                }
            }
        }
    }
}
