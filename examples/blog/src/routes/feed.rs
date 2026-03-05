use maudit::feed::{RssFeed, RssItem};
use maudit::route::prelude::*;

use crate::{
    content::ArticleContent,
    routes::article::ArticleParams,
    routes::Article,
};

#[route("/feed.xml")]
pub struct Feed;

impl Route for Feed {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let articles = ctx.content.get_source::<ArticleContent>("articles");
        let base = ctx.base_url.as_deref().unwrap_or("");

        RssFeed::new(
            "Maudit Example Blog",
            ctx.canonical_url()
                .unwrap_or_else(|| base.to_string()),
            "A sample blog built with Maudit.",
        )
        .language("en")
        .items(articles.entries.iter().map(|entry| {
            let data = entry.data(ctx);
            RssItem::new(
                data.title.clone(),
                format!("{}{}", base, Article.url(ArticleParams { article: entry.id.clone() })),
            )
            .description(&data.description)
        }))
    }
}
