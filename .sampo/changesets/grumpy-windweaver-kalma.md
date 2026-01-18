---
cargo/maudit: minor
---

Adds a new `redirect()` function that can used to generate redirects to other pages and websites. This function is exported from the route prelude.

```rs
use maudit::route::prelude::*;

#[route("/redirect")]
pub struct Redirect;

impl Route for Redirect {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        redirect("https://example.com")

        // Use a page's url method to generate type safe links:
        // redirect(&OtherPage.url(None))
    }
}
```
