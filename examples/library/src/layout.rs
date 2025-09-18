use maud::{html, Markup, PreEscaped};

pub fn layout(content: String) -> Markup {
    html! {
        html {
            head {
                meta charset="utf-8";
                title { "My Blog" }
            }
            body {
                header {
                    h1 { "My Blog" }
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
