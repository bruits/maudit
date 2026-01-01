use maud::{DOCTYPE, Markup, html};

pub fn layout(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "i18n Example" }
            }
            body {
                (content)
            }
        }
    }
}
