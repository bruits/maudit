use std::vec;

use maudit::{
    content::{ContentSource, ContentSources},
    page::prelude::*,
    trying::Res,
    FxHashMap,
};

use crate::{content::ArticleContent, layout::layout};

#[route("/articles/[article]")]
pub struct Article;

#[derive(Params)]
pub struct ArticleParams {
    pub article: String,
}

impl DynamicRoute for Article {
    fn routes(&self, ctx: &DynamicRouteContext) -> Vec<RouteParams> {
        //let articles = ctx.content.get_source::<ArticleContent>("articles");

        let mut params = FxHashMap::default();

        params.insert("article".to_string(), "first-post".to_string());

        vec![RouteParams(params)]
    }
}

impl Article {
    fn render(content: Res<ContentSources>) -> RenderResult {
        let articles = content.get_source::<ArticleContent>("articles");
        let article = articles.get_entry("first-post");

        layout((article.render)()).into()
    }
}
