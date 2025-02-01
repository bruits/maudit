---
title: "Assets"
description: "Welcome to the Maudit documentation!"
section: "core-concepts"
---

Maudit supports importing assets like images, stylesheets, and scripts into your project and pages.

### Images

To import an image, add it anywhere in your project's directory, and use the `ctx.assets.add_image()` method to add it to a page's assets.

Like other assets, images can be used directly in Maud templates.

```rust
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

```rust
fn render(&self, ctx: &mut RouteContext) -> RenderResult {
  let image = ctx.assets.add_image("logo.png");

  RenderResult::Html(format!("<img src=\"{}\" loading=\"lazy\" decoding=\"async\" />", image.url().unwrap()))
}
```

At this time, images are not automatically optimized or resized, but this will be added in the future.

### Stylesheets

To import a stylesheet, add it anywhere in your project's directory, and use the `ctx.assets.add_style()` method to add it to a page's assets.

In [supported templating languages](/docs/templating/), the return value of `ctx.assets.add_style()` can be used directly in the template.

```rust
use maudit::page::prelude::*;
use maud::{html, Markup};

#[route("/blog")]
pub struct Blog;

impl Page<Markup> for Blog {
  fn render(&self, ctx: &mut RouteContext) -> Markup {
    let style = ctx.assets.add_style("style.css", false);

    html! {
      (style) // Generates <link rel="stylesheet" href="STYLE_URL" />
    }
  }
}
```

Alternatively, the `include_style()` method can be used to automatically include the stylesheet in the page, without needing to manually add it to the template. Note that, at this time, pages without a `head` tag won't have the stylesheet included.

```rust
fn render(&self, ctx: &mut RouteContext) -> Markup {
  ctx.assets.include_style("style.css", false);

  html! {
    div {
      "Look ma, no link tag!"
    }
  }
}
```

#### Tailwind support

Maudit includes built-in support for [Tailwind CSS](https://tailwindcss.com/). To use it, pass `true` as the second argument to `add_style()` or `include_style()`. In the future, Maudit will automatically detect Tailwind CSS and enable it when needed.

```rust
fn render(&self, ctx: &mut RouteContext) -> Markup {
  ctx.assets.add_style("style.css", true);

  html! {
    div.bg-red-500 {
      "Wow, such red!"
    }
  }
}
```

Tailwind can then be configured normally, through native CSS in Tailwind 4.0, or through a `tailwind.config.js` file in earlier versions.

**Caution:** Tailwind CSS is a JavaScript-based tool, which means that Maudit needs to spawn a separate Node.js process to run it. This comes with a significant performance overhead and in most projects using it, Tailwind will account for more than 99% of the build time, even when using Tailwind 4.0.

### Scripts

JavaScript and TypeScript files can be added to pages using the `ctx.assets.add_script()` method.

```rust
use maudit::page::prelude::*;
use maud::{html, Markup};

#[route("/blog")]
pub struct Blog;

impl Page<Markup> for Blog {
  fn render(&self, ctx: &mut RouteContext) -> Markup {
    let script = ctx.assets.add_script("script.js");

    html! {
      (script) // Generates <script src="SCRIPT_URL" type="module"></script>
    }
  }
}
```

The `include_script()` method can be used to automatically include the script in the page, which can be useful when using layouts or other shared templates.

```rust
fn render(&self, ctx: &mut RouteContext) -> Markup {
  ctx.assets.include_script("script.js");

  html! {
    div {
      "Look ma, no explicit script tag!"
    }
  }.into()
}
```

When using `include_script()`, the script will be included inside the `head` tag with the `type="module"` attribute. [Note that this attribute implicitely means that your script will be deferred](https://v8.dev/features/modules#defer) after the page has loaded. At this time, pages without a `head` tag won't have the script included.

### Transformation & Bundling

Maudit uses [Rolldown](https://rolldown.rs) to process and bundle scripts and styles. Rolldown will automatically chunk, minify, transpile, etc. your scripts and stylesheets, optimizing them for production. Features like tree shaking, minification, TypeScript support and more are all included out of the box.

At this time, Maudit does not support customizing the transformation process, but this will be added in the future.
