use maud::html;
use maudit::route::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::content::ArticleContent;

/// Route that lists all articles - uses `entries()` which tracks all content files
#[route("/articles")]
pub struct Articles;

impl Route for Articles {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let articles = ctx.content.get_source::<ArticleContent>("articles");

        // Using entries() tracks ALL content files in the source
        let article_list: Vec<_> = articles.entries().iter().collect();

        // Generate a unique build ID - uses nanoseconds for uniqueness
        let build_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "0".to_string());

        html! {
            html {
                head {
                    title { "Articles" }
                }
                body data-build-id=(build_id) {
                    h1 id="title" { "Articles" }
                    ul id="article-list" {
                        @for article in article_list {
                            li {
                                a href=(format!("/articles/{}", article.id)) {
                                    (article.data(ctx).title)
                                }
                                " - "
                                (article.data(ctx).description)
                            }
                        }
                    }
                }
            }
        }
    }
}
