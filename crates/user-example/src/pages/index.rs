use maudit::assets::Asset;
use maudit::maud::html;
use maudit::maudit_macros::route;

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
