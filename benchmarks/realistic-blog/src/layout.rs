use maud::{html, Markup, PreEscaped};
use maudit::route::PageContext;

pub fn layout(ctx: &mut PageContext, content: String) -> Markup {
    ctx.assets.include_style("src/style.css");

    html! {
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Erika's Super Blog" }
            }
            body {
                header {
                    h1 { a href="/" { "Erika's Super Blog" } }
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
