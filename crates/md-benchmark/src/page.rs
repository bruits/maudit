use maud::{html, PreEscaped};
use maudit::{content::UntypedMarkdownContent, page::prelude::*};

#[route("/[file]")]
pub struct Article;

#[derive(Params)]
struct Params {
    file: String,
}

impl DynamicRoute<Params> for Article {
    fn routes(&self, context: &mut DynamicRouteContext) -> Vec<Params> {
        context
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .into_params(|entry| Params {
                file: entry.id.clone(),
            })
    }
}

impl Page for Article {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params::<Params>();
        let entry = ctx
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .get_entry(params.file.as_str());

        let content = PreEscaped(entry.render());
        html!((content)).into()
    }
}
