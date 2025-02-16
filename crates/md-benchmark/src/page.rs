use maud::{html, PreEscaped};
use maudit::{content::UntypedMarkdownContent, page::prelude::*};

#[route(self.route)]
pub struct Article {
    pub route: String,
}

#[derive(Params)]
struct Params {
    file: String,
}

impl Page<Params> for Article {
    fn routes(&self, context: &mut DynamicRouteContext) -> Vec<Params> {
        context
            .content
            .get_source::<UntypedMarkdownContent>("articles")
            .into_params(|entry| Params {
                file: entry.id.clone(),
            })
    }

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
