---
title: "Scripts"
description: "Learn how to import and use JavaScript and TypeScript files in your Maudit site."
section: "core-concepts"
---

Maudit supports adding JavaScript and TypeScript files to your site.

To import a script, add it anywhere in your project's directory, and use the [`ctx.assets.add_script()`](https://docs.rs/maudit/latest/maudit/assets/struct.RouteAssets.html#method.add_script) method to add it to a page's assets.

This function will return an error if the image file does not exist, or cannot be read for any reason. If you'd rather not deal with errors, you can use the `add_script_unchecked()` method, which will instead panic on failure.

```rs
use maudit::route::prelude::*;
use maud::{html, Markup};

#[route("/")]
pub struct Index;

impl Route for Index {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let script = ctx.assets.add_script("script.js")?;

    // Access the URL of the script using the `url()` method.
    // This is useful when you want to manually add the script to your template.
    format!(
      r#"<script src="{}" type="module"></script>"#,
      script.url()
    );

    // In supported templating languages, the return value of `ctx.assets.add_script()` can be used directly in the template.
    Ok(html! {
      (script) // Generates <script src="SCRIPT_URL" type="module"></script>
    })
  }
}
```

Alternatively, the [`include_script()`](https://docs.rs/maudit/latest/maudit/assets/struct.RouteAssets.html#method.include_script) method can be used to automatically include the script in the page, without needing to manually add it to the template. This can be useful when a layout or component need to include their own scripts.

```rs
fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
  layout(&ctx, "Look ma, no explicit script tag!")
}

fn layout(ctx: &PageContext, content: &str) -> impl Into<RenderResult> {
  ctx.assets.include_script("script.js")?;

  Ok(html! {
    head {
      title { "My page" }
      // No need to manually add the script here.
    }
    body {
      (PreEscaped(content))
    }
  })
}
```

When using `include_script()`, the script will be included inside the `head` tag with the `type="module"` attribute. [Note that this attribute implicitely means that your script will be deferred](https://v8.dev/features/modules#defer) after the page has loaded. Note that, at this time, pages without a `head` tag won't have the script included.

In both cases, paths are relative to the project's current working directory, not the file where the method is called. It is possible to resolve relatively to the current file using Rust's [`file!()`](https://doc.rust-lang.org/std/macro.file.html) macro, if needed:

```rs
let script = ctx.assets.add_script(
  std::path::Path::new(file!())
    .parent()
    .unwrap()
    .join("script.js")
    .to_str()
    .unwrap(),
)?;
```

## Transformation & Bundling

Maudit uses [Rolldown](https://rolldown.rs) to process and bundle scripts and styles. Rolldown will automatically chunk, minify, transpile, etc. your scripts and stylesheets, optimizing them for production. Features like tree shaking, minification, TypeScript support and more are all included out of the box.

At this time, Maudit does not support customizing the transformation process, but this will be added in the future.
