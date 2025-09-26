use maud::html;
use maudit::route::prelude::*;

use crate::{content::ArticleContent, layout::layout};

#[route("/articles/[page]")]
pub struct Articles;

#[derive(Params, Clone)]
pub struct ArticlesParams {
    pub page: Option<usize>,
}

impl Route<ArticlesParams, PaginatedContentPage<ArticleContent>> for Articles {
    fn pages(
        &self,
        ctx: &mut DynamicRouteContext,
    ) -> Pages<ArticlesParams, PaginatedContentPage<ArticleContent>> {
        let articles = &ctx.content.get_source::<ArticleContent>("articles").entries;

        let mut articles = articles.to_vec();
        articles.sort_by(|a, b| b.data(ctx).date.cmp(&a.data(ctx).date));

        paginate(articles, 4, |page| ArticlesParams {
            page: if page == 0 { None } else { Some(page) },
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let current_page = ctx.params::<ArticlesParams>().page.unwrap_or(0);
        let props = ctx.props::<PaginatedContentPage<ArticleContent>>();

        let markup = html! {
          ul.articles-list {
            @for entry in &props.items {
              li {
                a href=(&Article.url(ArticleParams { article: entry.id.clone() })) {
                    h2 { (entry.data(ctx).title) }
                }
                p { (entry.data(ctx).description) }
                span { (entry.data(ctx).date) }
              }
            }
          }
          div.article-pagination {
            @if props.has_next {
                @let next_page = current_page + 1;
                @let next_param = if next_page == 0 { None } else { Some(next_page) };
                a href=(&Articles.url(ArticlesParams { page: next_param })) { "Previous page..." }
            }
            @if props.has_prev {
                @let prev_page = current_page.saturating_sub(1);
                @let prev_param = if prev_page == 0 { None } else { Some(prev_page) };
                a href=(&Articles.url(ArticlesParams { page: prev_param })) { "Next page..." }
            }
        }
        }
        .into_string();

        layout(ctx, markup)
    }
}

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
            Page::from_params(ArticleParams {
                article: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<ArticleParams>();
        let articles = ctx.content.get_source::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);

        let content = article.render(ctx);
        layout(ctx, content)
    }
}
