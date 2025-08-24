---
title: "Styling"
description: "Learn how to style your Maudit site."
section: "core-concepts"
---

Maudit supports styling your site with CSS.

To import a stylesheet, add it anywhere in your project's directory, and use the `ctx.assets.add_style()` method to add it to a page's assets.

In [supported templating languages](/docs/templating/), the return value of `ctx.assets.add_style()` can be used directly in the template.

```rs
use maudit::page::prelude::*;
use maud::{html, Markup};

#[route("/blog")]
pub struct Blog;

impl Page<RouteParams, Markup> for Blog {
  fn render(&self, ctx: &mut RouteContext) -> Markup {
    let style = ctx.assets.add_style("style.css", false);

    html! {
      (style) // Generates <link rel="stylesheet" href="STYLE_URL" />
    }
  }
}
```

Alternatively, the `include_style()` method can be used to automatically include the stylesheet in the page, without needing to manually add it to the template. Note that, at this time, pages without a `head` tag won't have the stylesheet included.

```rs
fn render(&self, ctx: &mut RouteContext) -> Markup {
  ctx.assets.include_style("style.css", false);

  layout(
    html! {
      div {
        "Look ma, no link tag!"
      }
    }
  )
}
```

#### Tailwind support

Maudit includes built-in support for [Tailwind CSS](https://tailwindcss.com/). To use it, pass `true` as the second argument to `add_style()` or `include_style()`. In the future, Maudit will automatically detect Tailwind CSS and enable it when needed.

```rs
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

**Caution:** Tailwind CSS is a JavaScript-based tool, which means that Maudit needs to spawn a separate Node.js process to run it. This comes with a significant performance overhead and in most projects using it, Tailwind will account for more than 99% of the build time, even when using Tailwind 4.0+.
