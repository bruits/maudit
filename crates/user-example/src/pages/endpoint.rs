use maudit::maudit_macros::route;
use maudit::page::{Page, RenderResult, RouteContext};

#[route("/catalogue/data.json")]
pub struct Endpoint;

impl Page for Endpoint {
    fn render(&self, _context: &RouteContext) -> RenderResult {
        // Return some JSON
        RenderResult::Text(r#"{"message": "Hello, world!"}"#.to_string())
    }
}
