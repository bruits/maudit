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
        let articles = ctx.content.get_collection::<ArticleContent>("articles");
        let mut static_routes: Vec<ArticleParams> = vec![];

        for article in &articles.entries {
            static_routes.push(ArticleParams {
                article: article.id.clone(),
            });
        }

        RouteParams::from_vec(static_routes)
    }
}

impl Page for Article {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params.parse_into::<ArticleParams>();
        let articles = ctx.content.get_collection::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);

        layout((article.render)()).into()
    }
}
