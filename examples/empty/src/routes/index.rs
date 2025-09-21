use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, _ctx: &mut PageContext) -> RenderResult {
        "Hello, world!".into()
    }
}
