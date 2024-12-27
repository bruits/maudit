use maud::{html, Markup, PreEscaped};

pub fn layout(content: String) -> Markup {
    html! {
        html {
            head {
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
