use maudit::route::prelude::*;

use maud::html;

#[route("/[page]")]
pub struct DynamicExample;

#[derive(Params, Clone)]
pub struct Params {
    pub page: u128,
}

impl Route<Params> for DynamicExample {
    fn pages(&self, _: &mut DynamicRouteContext) -> Pages<Params> {
        (0..1)
            .map(|i| Page::from_params(Params { page: i }))
            .collect()
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<Params>();
        let image = ctx.assets.add_image("data/social-card.png");
        ctx.assets
            .include_style_with_options("data/tailwind.css", StyleOptions { tailwind: true });

        html! {
            head {
                title { "Index" }
            }
            h1 { "Hello, world!" }
            (image.render("Maudit social card, a crudely drawn crown"))
            p { (params.page) }
        }
    }
}
