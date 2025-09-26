use maudit::route::prelude::*;

#[route("/[page]")]
pub struct Article;

#[derive(Params, Clone)]
struct Params {
    page: u16,
}

impl Route<Params> for Article {
    fn pages(&self, _: &mut DynamicRouteContext) -> Vec<Page<Params>> {
        (0..10000)
            .map(|i| Page::new(Params { page: i }, ()))
            .collect()
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<Params>();

        let mut buffer = itoa::Buffer::new();
        let page_str = buffer.format(params.page);

        page_str.to_string()
    }
}
