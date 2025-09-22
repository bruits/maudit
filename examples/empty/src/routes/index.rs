use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, _: &mut PageContext) -> impl Into<RenderResult> {
        "Hello, world!"
    }
}
