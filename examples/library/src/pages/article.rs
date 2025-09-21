use maudit::page::prelude::*;

use crate::{content::ArticleContent, layout::layout};

#[route("/articles/[article]")]
pub struct Article;

#[derive(Params, Clone)]
pub struct ArticleParams {
    pub article: String,
}

impl Route<ArticleParams> for Article {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<ArticleParams> {
        let articles = ctx.content.get_source::<ArticleContent>("articles");

        articles.into_pages(|entry| {
            Route::from_params(ArticleParams {
                article: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        let params = ctx.params::<ArticleParams>();
        let articles = ctx.content.get_source::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);

        let headings = article.data(ctx).get_headings();
        println!("{:?}", headings);

        layout(article.render(ctx)).into()
    }
}
