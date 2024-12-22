use maudit::assets::Asset;
use maudit::maud::html;
use maudit::maudit_macros::route;
use maudit::page::RouteContext;

#[route("/[page]")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &RouteContext) -> RenderResult {
        let social_card = Asset::new("./data/social-card.png".into());

        let params = ctx.params.get("page").unwrap();

        RenderResult::Html(html! {
          h1 { "Hello, world!" }
          img src=(social_card) alt="Social card";
                    p { (params) }
        })
    }
}

impl DynamicPage for Index {
    fn routes(&self) -> std::collections::HashMap<String, String> {
        let mut routes = std::collections::HashMap::new();
        for i in 1..=100 {
            routes.insert("page".to_string(), format!("Hello {}", i));
        }
        routes
    }
}
