use maud::html;
use maudit::page::prelude::*;

#[route("/chat")]
pub struct ChatRedirect;

pub const DISCORD_INVITE: &str = "https://discord.gg/84pd4QtmzA";

impl Page for ChatRedirect {
    fn render(&self, _: &mut RouteContext) -> RenderResult {
        html! {
            head {
                meta http-equiv="refresh" content=(format!("0;url={}", DISCORD_INVITE));
            }
        }
        .into()
    }
}
