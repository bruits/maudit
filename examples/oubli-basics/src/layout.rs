use maud::{DOCTYPE, Markup, html};

pub fn layout(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Test page" }
            }
            body {
                (content)
            }
        }
    }
}
