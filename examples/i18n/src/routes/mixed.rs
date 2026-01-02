use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[derive(Params, Clone)]
pub struct MixedParams {
    pub id: String,
}

// Base route is static (/products)
// But variants have dynamic parameters (/en/products/[id])
#[route(locales(en = "/en/products/[id]", sv = "/sv/produkter/[id]"))]
pub struct Mixed;

impl Route<MixedParams> for Mixed {
    fn pages(&self, _ctx: &mut DynamicRouteContext) -> Pages<MixedParams> {
        vec![
            Page::from_params(MixedParams {
                id: "laptop".to_string(),
            }),
            Page::from_params(MixedParams {
                id: "phone".to_string(),
            }),
        ]
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<MixedParams>();

        layout(html! {
            h1 { "Product: " (params.id) }
            p { "This route has a static base path but dynamic variants!" }
            nav {
                ul {
                    li { a href="/en/products/laptop" { "English - Laptop" } }
                    li { a href="/en/products/phone" { "English - Phone" } }
                    li { a href="/sv/produkter/laptop" { "Swedish - Laptop" } }
                    li { a href="/sv/produkter/phone" { "Swedish - Phone" } }
                }
            }
        })
    }
}
