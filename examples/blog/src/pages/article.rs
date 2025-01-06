use maud::Markup;
use maudit::page::prelude::*;

use crate::{content::ArticleContent, layout::layout};

#[route("/articles/[article]")]
pub struct Article;

#[derive(Params)]
pub struct ArticleParams {
    pub article: String,
}

impl DynamicRoute<ArticleParams> for Article {
    fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<ArticleParams> {
        let articles = ctx.content.get_source::<ArticleContent>("articles");

        articles.into_params(|entry| ArticleParams {
            article: entry.id.clone(),
        })
    }
}

impl Page<Markup> for Article {
    fn render(&self, ctx: &mut RouteContext) -> Markup {
        let params = ctx.params::<ArticleParams>();
        let articles = ctx.content.get_source::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);

        let headings = article.data.get_headings();
        println!("{:?}", headings);

        layout(article.render())
    }
}
