use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[derive(Params, Clone)]
pub struct ArticleParams {
    pub slug: String,
}

#[route(
    "/articles/[slug]",
    locales(en = "/en/articles/[slug]", sv = "/sv/artiklar/[slug]")
)]
pub struct Article;

impl Route<ArticleParams> for Article {
    fn pages(&self, _ctx: &mut DynamicRouteContext) -> Pages<ArticleParams> {
        vec![
            Page::from_params(ArticleParams {
                slug: "hello-world".to_string(),
            }),
            Page::from_params(ArticleParams {
                slug: "getting-started".to_string(),
            }),
        ]
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<ArticleParams>();

        let variant_info = if let Some(variant) = &ctx.variant {
            format!("Variant: {}", variant)
        } else {
            "Base route (no variant)".to_string()
        };

        layout(html! {
            h1 { "Article: " (params.slug) }
            p { (variant_info) }
            p { "This is a dynamic route with localized variants." }
            nav {
                ul {
                    li { a href="/articles/hello-world" { "Default" } }
                    li { a href="/en/articles/hello-world" { "English" } }
                    li { a href="/sv/artiklar/hello-world" { "Swedish" } }
                }
            }
        })
    }
}
