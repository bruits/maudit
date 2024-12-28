use maudit::page::prelude::*;

use crate::{content::ArticleContent, layout::layout};

#[route("/articles/[article]")]
pub struct Article;

#[derive(Params)]
pub struct ArticleParams {
    pub article: String,
}

impl DynamicPage for Article {
    fn routes(&self, ctx: &DynamicRouteContext) -> Vec<RouteParams> {
        let collection = ctx.content.get_collection::<ArticleContent>("articles");

        collection.into_params(|entry| ArticleParams {
            article: entry.id.clone(),
        })
    }
}

impl Page for Article {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params::<ArticleParams>();
        let articles = ctx.content.get_collection::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);

        layout((article.render)()).into()
    }
}
