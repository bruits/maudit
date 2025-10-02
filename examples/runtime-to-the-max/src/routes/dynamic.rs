use maudit::route::prelude::*;

#[route(format!("/dynamic/{}/", self.dynamic_page.0))]
pub struct Dynamic {
    pub dynamic_page: (String, String),
}

impl Route for Dynamic {
    fn render(&self, _: &mut PageContext) -> impl Into<RenderResult> {
        self.dynamic_page.1.clone()
    }
}
