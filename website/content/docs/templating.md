---
title: "Templating"
description: "Learn how to render content using your favorite templating engine."
section: "core-concepts"
---

In general, if a library can return a `String` or a `Vec<u8>`, you can use it to generate pages with Maudit.

Through crate features, Maudit includes built-in helper methods and traits implementation for popular templating libraries.

## Maud

Maudit implements `Into<RenderResult>` for the Maud `Markup` type, allowing one to directly return Maud's templates from a route's `render` method.

```rs
use maud::{html, Markup};
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, _: &mut PageContext) -> impl Into<RenderResult> {
        html! {
            h1 { "Hello, world!" }
        }
    }
}
```

Maudit implements the `Render` trait for images, scripts and styles, allowing one to use them directly in Maud templates.

```rs
use maud::{html, Markup};
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let style = ctx.add_style("style.css");
        let image = ctx.add_image("logo.png");

        html! {
            (style) // Will render to a <link> tag for the CSS file
            (image.render("The logo of my product")) // Will render to an <img> tag for the image
        }
    }
}
```

To use Maud with Maudit, install Maud into your project by adding it to your `Cargo.toml`, or running `cargo add maud`.

```toml
[dependencies]
maud = "0.27"
```

The `maud` feature is enabled by default. If you have disabled default features, you can enable it manually by editing your `Cargo.toml`:

```toml
maudit = { version = "...", features = ["maud"] }
```
