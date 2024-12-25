use maudit::page::prelude::*;

use maud::html;

#[route("/[page]")]
pub struct DynamicExample;

#[derive(Params)]
pub struct Params {
    pub page: u128,
}

impl DynamicPage for DynamicExample {
    fn routes(&self) -> Vec<RouteParams> {
        let mut static_routes: Vec<Params> = vec![];

        for i in 0..1 {
            static_routes.push(Params { page: i });
        }

        RouteParams::from_vec(static_routes)
    }
}

impl Page for DynamicExample {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let params = ctx.params.parse_into::<Params>();
        let image = ctx.assets.add_image("data/social-card.png".into());
        let style = ctx.assets.add_style("data/tailwind.css".into(), true);

        RenderResult::Html(html! {
            head {
                title { "Index" }
                link rel="stylesheet" href=(style.url().unwrap()) {}
            }
            h1 { "Hello, world!" }
            img src=(image.path.to_string_lossy()) {}
            p { (params.page) }
        })
    }
}
