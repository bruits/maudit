use maudit::page::prelude::*;

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self, _ctx: &mut RouteContext) -> RenderResult {
        "Hello, world!".into()
    }
}
