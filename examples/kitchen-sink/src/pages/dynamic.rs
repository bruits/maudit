use maudit::page::{prelude::*, DynamicRouteContext};

use maud::html;

#[route("/[page]")]
pub struct DynamicExample;

#[derive(Params)]
pub struct Params {
    pub page: u128,
}

impl DynamicRoute for DynamicExample {
    fn routes(&self, _: &DynamicRouteContext) -> Vec<RouteParams> {
        (0..1).map(|i| Params { page: i }.into()).collect()
    }
}

impl Page for DynamicExample {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params::<Params>();
        let image = ctx.assets.add_image("data/social-card.png");
        ctx.assets.include_style("data/tailwind.css", true);

        html! {
            head {
                title { "Index" }
            }
            h1 { "Hello, world!" }
            (image)
            p { (params.page) }
        }
        .into()
    }
}
