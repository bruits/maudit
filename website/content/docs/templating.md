---
title: "Templating"
description: "Learn how to render content using your favorite templating engine."
section: "core-concepts"
---

In general, if a library can return a String, you can use it to generate pages with Maudit.

Through crate features, Maudit includes built-in helper methods and traits implementation for popular templating libraries.

## Maud

Maudit implements `Into<RenderResult>` for the Maud `Markup` type, allowing one to directly return Maud's templates from a page's `render` method.

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

Maudit implements the `Render` trait for assets, such as scripts, styles, and images, allowing one to use them directly in Maud templates.

```rs
use maud::{html, Markup};
use maudit::route::prelude::*;

#[route("/")]
pub struct Index;

impl Route for Index {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let logo = ctx.add_image("./logo.png");

        html! {
            (logo) // Will generate <img src="IMAGE_PATH" width="IMAGE_WIDTH" height="IMAGE_HEIGHT" loading="lazy" decoding="async" />
        }
    }
}
```

This is made possible by the `maud` feature, which is enabled by default, but can be disabled using `default-features = false` in your `Cargo.toml` for `maudit`.
