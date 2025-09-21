---
title: "Scripts"
description: "Learn how to import and use JavaScript and TypeScript files in your Maudit site."
section: "core-concepts"
---

JavaScript and TypeScript files can be added to pages using the `ctx.assets.add_script()` method.

```rs
use maudit::route::prelude::*;
use maud::{html, Markup};

#[route("/blog")]
pub struct Blog;

impl Route<PageParams, Markup> for Blog {
  fn render(&self, ctx: &mut PageContext) -> Markup {
    let script = ctx.assets.add_script("script.js");

    html! {
      (script) // Generates <script src="SCRIPT_URL" type="module"></script>
    }
  }
}
```

The `include_script()` method can be used to automatically include the script in the page, which can be useful when using layouts or other shared templates.

```rs
fn render(&self, ctx: &mut PageContext) -> Markup {
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

## Transformation & Bundling

Maudit uses [Rolldown](https://rolldown.rs) to process and bundle scripts and styles. Rolldown will automatically chunk, minify, transpile, etc. your scripts and stylesheets, optimizing them for production. Features like tree shaking, minification, TypeScript support and more are all included out of the box.

At this time, Maudit does not support customizing the transformation process, but this will be added in the future.
