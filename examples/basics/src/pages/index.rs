use crate::layout::layout;
use maud::html;
use maudit::{page::prelude::*, trying::Res};

#[route("/")]
impl Index {
    pub fn render(current_url: Res<String>) -> RenderResult {
        println!("{:?}", current_url.value);

        layout(html! {
            // (logo)
            h1 { "Hello World" }
        })
        .into()
    }
}
