use maudit::{content::UntypedMarkdownContent, page::prelude::*};

#[route("/[file]")]
pub struct Article;

#[derive(Params, Clone)]
struct Params {
    file: String,
}

impl Route<Params> for Article {
    fn pages(&self, context: &mut DynamicRouteContext) -> Pages<Params> {
        context
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .into_pages(|entry| {
                Page::from_params(Params {
                    file: entry.id.clone(),
                })
            })
    }

    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        let params = ctx.params::<Params>();
        let entry = ctx
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .get_entry(params.file.as_str());

        entry.render(ctx).into()
    }
}
