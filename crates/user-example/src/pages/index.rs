use dire_coronet::assets::Asset;
use dire_coronet::dire_coronet_macros::route;
use dire_coronet::maud::html;

#[route("/")]
pub struct Index;

impl Page for Index {
    fn render(&self) -> RenderResult {
        let social_card = Asset::new("./data/social-card.png".into());

        RenderResult::Html(html! {
          h1 { "Hello, world!" }
          img src=(social_card) alt="Social card";
        })
    }
}
