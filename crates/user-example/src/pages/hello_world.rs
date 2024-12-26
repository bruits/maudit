use maud::html;
use maudit::page::prelude::*;

#[route("/hello-world")]
pub struct HelloWorld;

impl Page for HelloWorld {
    fn render(&self, _: &mut RouteContext) -> RenderResult {
        html! {
          h1 { "Hello World" }
        }
        .into()
    }
}
