---
title: "Assets"
description: "Learn how to import and use assets in your Maudit site."
section: "core-concepts"
---

Maudit supports importing assets like images, stylesheets, and scripts into your project and pages.

### Images

To import an image, add it anywhere in your project's directory, and use the `ctx.assets.add_image()` method to add it to a page's assets.

Like other assets, images can be used directly in Maud templates.

```rs
use maudit::page::prelude::*;
use maud::html;

#[route("/blog")]
pub struct Blog;

impl Page for Blog {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let image = ctx.assets.add_image("logo.png");

    html! {
      (image) // Generates <img src="IMAGE_URL" loading="lazy" decoding="async" />
    }.into()
  }
}
```

Alternatively, if not using Maud, the `url()` method on the image can be used to generate any necessary HTML or other output.

```rs
fn render(&self, ctx: &mut RouteContext) -> RenderResult {
  let image = ctx.assets.add_image("logo.png");

  RenderResult::Html(format!("<img src=\"{}\" loading=\"lazy\" decoding=\"async\" />", image.url().unwrap()))
}
```

At this time, images are not automatically optimized or resized, but this will be added in the future.

### Scripts

JavaScript and TypeScript files can be added to pages using the `ctx.assets.add_script()` method.

```rs
use maudit::page::prelude::*;
use maud::{html, Markup};

#[route("/blog")]
pub struct Blog;

impl Page<RouteParams, Markup> for Blog {
  fn render(&self, ctx: &mut RouteContext) -> Markup {
    let script = ctx.assets.add_script("script.js");

    html! {
      (script) // Generates <script src="SCRIPT_URL" type="module"></script>
    }
  }
}
```

The `include_script()` method can be used to automatically include the script in the page, which can be useful when using layouts or other shared templates.

```rs
fn render(&self, ctx: &mut RouteContext) -> Markup {
  ctx.assets.include_script("script.js");

  layout(
    html! {
      div {
        "Look ma, no explicit script tag!"
      }
    }
  )
}
```

When using `include_script()`, the script will be included inside the `head` tag with the `type="module"` attribute. [Note that this attribute implicitely means that your script will be deferred](https://v8.dev/features/modules#defer) after the page has loaded. At this time, pages without a `head` tag won't have the script included.

### Transformation & Bundling

Maudit uses [Rolldown](https://rolldown.rs) to process and bundle scripts and styles. Rolldown will automatically chunk, minify, transpile, etc. your scripts and stylesheets, optimizing them for production. Features like tree shaking, minification, TypeScript support and more are all included out of the box.

At this time, Maudit does not support customizing the transformation process, but this will be added in the future.
