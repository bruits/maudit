use maudit::{content::UntypedMarkdownContent, page::prelude::*};

#[route("/[file]")]
pub struct Article;

#[derive(Params)]
struct Params {
    file: String,
}

impl Page<Params> for Article {
    fn routes(&self, context: &mut DynamicRouteContext) -> Vec<Route<Params>> {
        context
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .into_routes(|entry| {
                Route::from_params(Params {
                    file: entry.id.clone(),
                })
            })
    }

    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params::<Params>();
        let entry = ctx
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .get_entry(params.file.as_str());

        entry.render(ctx).into()
    }
}
