use crate::layout::layout;
use maud::html;
use maudit::page::prelude::*;

#[route("/")]
impl Index {
    pub fn render(current_url: String) -> RenderResult {
        println!("{:?}", current_url);

        layout(html! {
            // (logo)
            h1 { "Hello World" }
        })
        .into()
    }
}
