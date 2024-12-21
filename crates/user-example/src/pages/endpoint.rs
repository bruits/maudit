use maudit::maudit_macros::route;

#[route("/catalogue/data.json")]
pub struct Endpoint;

impl Page for Endpoint {
    fn render(&self) -> RenderResult {
        // Return some JSON
        RenderResult::Text(r#"{"message": "Hello, world!"}"#.to_string())
    }
}
