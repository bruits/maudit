use crate::layout::layout;
use maud::html;
use maudit::route::prelude::*;

#[route("/another")]
pub struct Another;

impl Route for Another {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        Ok(layout(html! {
            h1 { "Hello World2" }
        }))
    }
}
