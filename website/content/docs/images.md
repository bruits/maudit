---
title: "Images"
description: "Learn how to import and use images in your Maudit site."
section: "core-concepts"
---

Maudit includes support using images in various contexts. In your pages as img tags, collocated next to and linked in your Markdown files, or inside JSON files, and more.

Additionally, Maudit supports processing (i.e. resizing and converting) images at build time.

## Using images

### In pages

To use an image in a page, add it anywhere in your project's directory, and use the [`ctx.assets.add_image()`](https://docs.rs/maudit/latest/maudit/assets/struct.RouteAssets.html#method.add_image) method to add it to a page's assets. This function returns a Result containing an [Image](https://docs.rs/maudit/latest/maudit/assets/struct.Image.html) instance, which contains information about the image, such as its URL, dimensions, and a method to render an img tag.

This function will error if the image file does not exist, or cannot be read for any reason. If you'd rather not deal with errors, you can use the `add_image_unchecked()` method, which will instead panic on failure.

```rs
use maudit::route::prelude::*;
use maud::{html};

#[route("/blog")]
pub struct Blog;

impl Route for Blog {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let image = ctx.assets.add_image("logo.png")?;

    let (width, height) = image.dimensions();
    format!("<img src=\"{}\" alt=\"My logo\" width=\"{}\" height=\"{}\" />", image.url(), width, height);

    // A more convenient way to render an image is to use the `render()` method, which generates an img tag for you and enforces accessibility by requiring an alt text.
    Ok(format!("{}", image.render("The logo of my project, a stylized crown")))
  }
}
```

Paths to image are resolved relative to the root of your project, not from the page's location, as such `./image.png` and `image.png` both refer to the same file in the project root.

### In Markdown

To use an image in Markdown, link to it using standard Markdown syntax.

```markdown
![Description](./image.png)
```

Images can be collocated next to your content, or anywhere else in your project and are resolved relatively to your Markdown file.

## Processing images

Images added to pages can be transformed by using [`ctx.assets.add_image_with_options()`](https://docs.rs/maudit/latest/maudit/assets/struct.RouteAssets.html#method.add_image_with_options), which takes an additional [`ImageOptions`](https://docs.rs/maudit/latest/maudit/assets/struct.ImageOptions.html) struct to specify how the image should be processed.

```rs
use maud::html;
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
        format: Some(ImageFormat::Png),
      },
    )?;

    Ok(html! {
      (image.render("My 800 pixel wide PNG"))
    })
  }
}
```

Processing images in Markdown files using the standard syntax is currently not supported, but can be achieved using a custom [shortcode](/docs/content/#shortcodes) or [component](/docs/content/#components).

## Placeholders

Maudit supports generating low-quality image placeholders (LQIP) for images. This can be useful to improve the perceived performance of your site by showing a blurred preview of an image while the full image is loading.

To generate a placeholder, use the [`placeholder()`](https://docs.rs/maudit/latest/maudit/assets/struct.Image.html#method.placeholder) method on an [Image](https://docs.rs/maudit/latest/maudit/assets/struct.Image.html) instance, for example returned by `ctx.assets.add_image()` or `ctx.assets.add_image_with_options()`.

It can then be included into the page by using the [`data_uri()`](https://docs.rs/maudit/latest/maudit/assets/struct.ImagePlaceholder.html#method.data_uri) method on the returned [`ImagePlaceholder`](https://docs.rs/maudit/latest/maudit/assets/struct.ImagePlaceholder.html) instance.

```rs
use maudit::route::prelude::*;

#[route("/image")]
pub struct ImagePage;

impl Route for ImagePage {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let image = ctx.assets.add_image("path/to/image.jpg")?;
    let placeholder = image.placeholder()?;

    Ok(format!("<img src=\"{}\" alt=\"Image with placeholder\" style=\"background-image: url('{}'); background-size: cover;\" />", image.url(), placeholder.data_uri()))
  }
}
```

Alternatively, it is possible to get the dominant colors of an image using the [`average_rgba()`](https://docs.rs/maudit/latest/maudit/assets/struct.ImagePlaceholder.html#method.average_rgba) method on the placeholder. This method will return a tuple of four `u8` values representing the red, green, blue, and alpha channels of the average color of the image.

The generation of placeholders is powered by the [ThumbHash](https://evanw.github.io/thumbhash/) library.
