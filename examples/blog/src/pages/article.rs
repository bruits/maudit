use maudit::page::prelude::*;

use crate::{content::ArticleContent, layout::layout};

#[route("/articles/[article]")]
pub struct Article;

#[derive(Params)]
pub struct ArticleParams {
    pub article: String,
}

impl DynamicRoute for Article {
    fn routes(&self, ctx: &DynamicRouteContext) -> Vec<RouteParams> {
        let articles = ctx.content.get_source::<ArticleContent>("articles");

        articles.into_params(|entry| ArticleParams {
            article: entry.id.clone(),
        })
    }
}

impl Page for Article {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params::<ArticleParams>();
        let articles = ctx.content.get_source::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);

        layout((article.render)()).into()
    }
}
