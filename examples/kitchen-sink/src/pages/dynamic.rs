use maudit::{
    page::{prelude::*, DynamicRouteContext},
    StyleOptions,
};

use maud::html;

#[route("/[page]")]
pub struct DynamicExample;

#[derive(Params)]
pub struct Params {
    pub page: u128,
}

impl Page<Params> for DynamicExample {
    fn routes(&self, _: &mut DynamicRouteContext) -> Vec<Params> {
        (0..1).map(|i| Params { page: i }).collect()
    }

    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params::<Params>();
        let image = ctx.assets.add_image("data/social-card.png");
        ctx.assets
            .include_style("data/tailwind.css", Some(StyleOptions { tailwind: true }));

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
