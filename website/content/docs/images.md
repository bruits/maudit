---
title: "Images"
description: "Learn how to import and use images in your Maudit site."
section: "core-concepts"
---

Maudit includes support using images in various contexts. In your pages as img tags, collocated next to and linked in your Markdown files, or inside JSON files, and more.

Additionally, Maudit supports processing (i.e. resizing and converting) images at build time.

## Using images

### In pages

To use an image in a page, add it anywhere in your project's directory, and use the `ctx.assets.add_image()` method to add it to a page's assets.

```rs
use maudit::route::prelude::*;

#[route("/blog")]
pub struct Blog;

impl Route for Blog {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let image = ctx.assets.add_image("logo.png");

    format!("", image.url)
  }
}
```

Paths to image are resolved relative to the root of your project, not from the page's location.

### In Markdown

To use an image in Markdown, link to it using standard Markdown syntax.

```markdown
![Description](./image.png)
```

Images can be collocated next to your content, or anywhere else in your project and are resolved relatively to your Markdown file.

## Processing images

Images added to pages can be transformed by using `ctx.assets.add_image_with_options()`.

```rs
use maudit::route::prelude::*;

#[route("/image")]
pub struct ImagePage;

impl Route for ImagePage {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let image = ctx.assets.add_image_with_options(
      "path/to/image.jpg",
      ImageOptions {
        width: Some(800),
        height: None,
        format: Some(ImageFormat::Png)
      },
    )?;

    format!("<img src=\"{}\" alt=\"Processed Image\" />", image.url)
  }
}
```

Processing images in Markdown files using the standard syntax is currently not supported, but can be achieved using a custom [shortcode](/docs/content/#shortcodes) or [component](/docs/content/#components).
