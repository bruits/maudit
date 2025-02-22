use maud::{html, Markup, PreEscaped};

pub fn layout(title: String, content: String) -> Markup {
    html! {
        html {
            head {
                meta charset="utf-8";
                title { (title) }
            }
            body {
                header {
                    h1 { (title) }
                }
                main {
                    (PreEscaped(content))
                }
                footer {
                    p { "© 2024 My Super Blog" }
                }
            }
        }
    }
}
