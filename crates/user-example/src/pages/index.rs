use maudit::maud::html;
use maudit::maudit_macros::route;
use maudit::page::RouteContext;

#[route("/[page]")]
pub struct Index;

impl Page for Index {
    fn render(&self, ctx: &RouteContext) -> RenderResult {
        let params = ctx.params.get("page").unwrap();

        RenderResult::Html(html! {
          h1 { "Hello, world!" }
          p { (params) }
        })
    }
}

impl DynamicPage for Index {
    fn routes(&self) -> Vec<std::collections::HashMap<String, String>> {
        // Return 100 routes
        (0..1000)
            .map(|i| {
                let mut map = std::collections::HashMap::new();
                map.insert("page".into(), i.to_string());
                map
            })
            .collect()
    }
}
