use maudit::{content::UntypedMarkdownContent, route::prelude::*};

#[route("/[file]")]
pub struct Article;

#[derive(Params, Clone)]
struct Params {
    file: String,
}

impl Route<Params> for Article {
    fn pages(&self, context: &mut DynamicRouteContext) -> Pages<Params> {
        let articles = context.content::<UntypedMarkdownContent>("articles");
        articles.into_pages(|entry| {
            Page::from_params(Params {
                file: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<Params>();
        let articles = ctx.content::<UntypedMarkdownContent>("articles");
        let entry = articles.get_entry(params.file.as_str());

        entry.render(ctx)
    }
}
