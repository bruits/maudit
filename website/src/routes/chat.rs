use maud::html;
use maudit::route::prelude::*;

#[route("/chat")]
pub struct ChatRedirect;

pub const DISCORD_INVITE: &str = "https://discord.gg/84pd4QtmzA";

impl Route for ChatRedirect {
    fn render(&self, _: &mut PageContext) -> RenderResult {
        html! {
            head {
                meta http-equiv="refresh" content=(format!("0;url={}", DISCORD_INVITE));
            }
        }
        .into()
    }
}
