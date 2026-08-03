---
title: "OpenGraph images"
description: "Learn how to generate OpenGraph images for your Maudit site."
section: "core-concepts"
---

[OpenGraph](https://ogp.me/) images are the previews shown when a page is shared on social networks and chat applications. Maudit can generate them at build time, either from SVG markup built per page, or from an image already in your project.

## Enabling the feature

OpenGraph image generation lives behind the `og_image` crate feature, which is **not** enabled by default, as it pulls in [resvg](https://github.com/linebender/resvg), an SVG rasterizer that is a fairly heavy dependency to compile. Enable it in your `Cargo.toml`:

```toml
maudit = { version = "...", features = ["og_image"] }
```

Generated images need an absolute URL, because OpenGraph consumers do not resolve relative ones. Set [`base_url`](https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html) to your site's URL, otherwise adding an OpenGraph image returns an error.

```rs
coronate(
  routes![/* ... */],
  content_sources![],
  BuildOptions {
    base_url: Some("https://example.com".into()),
    ..Default::default()
  },
)
```

## Generating an image from SVG

Pass SVG markup to [`ctx.assets.add_opengraph_image()`](https://docs.rs/maudit/latest/maudit/assets/struct.RouteAssets.html#method.add_opengraph_image) to render it to a PNG at build time. Since the SVG is just a string, it can be built per page, which is the most common way to produce a distinct preview for every article of a blog.

```rs
use maud::html;
use maudit::route::prelude::*;

#[route("/blog")]
pub struct Blog;

impl Route for Blog {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let title = "Hello, world!";

    let og_image = ctx.assets.add_opengraph_image(&format!(
      r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
            <rect width="100%" height="100%" fill="#1a1a1a"/>
            <text x="60" y="330" fill="#ffffff" font-size="72" font-family="sans-serif">{title}</text>
          </svg>"##
    ))?;

    Ok(html! {
      head {
        (og_image.render())
      }
    })
  }
}
```

The rendered PNG takes the dimensions of the SVG itself. Most social networks expect 1200x630 pixels.

Text is drawn with the fonts installed on the machine running the build, so prefer generic families such as `sans-serif`, or embed the glyphs you need in the SVG, if you want previews to look the same everywhere.

## Using an existing image

An [`Image`](https://docs.rs/maudit/latest/maudit/assets/struct.Image.html) can be passed instead of SVG markup, which is handy for a single pre-made preview shared by every page. Raster images (PNG, JPEG, WebP, …) are referenced as-is, and `.svg` files are rendered to a PNG like inline markup is.

```rs
let cover = ctx.assets.add_image("images/og-cover.png")?;
let og_image = ctx.assets.add_opengraph_image(&cover)?;
```

## Referencing the image

As with [images](/docs/images/), adding an OpenGraph image generates it, but does not reference it in the page. The [`render()`](https://docs.rs/maudit/latest/maudit/assets/struct.OpenGraphImage.html#method.render) method returns the `<meta>` tags to place in your `head`:

```html
<meta property="og:image" content="https://example.com/_maudit/og-image.a1b2c.png"/>
<meta property="og:image:type" content="image/png"/>
<meta property="og:image:width" content="1200"/>
<meta property="og:image:height" content="630"/>
```

The other OpenGraph tags, such as `og:title` and `og:description`, are up to you to write, as Maudit only handles the image. The absolute URL is also available on its own through the [`url()`](https://docs.rs/maudit/latest/maudit/assets/struct.OpenGraphImage.html#method.url) method, for instance to reuse it in a `twitter:image` tag.

Rendering an SVG is expensive, so generated images are cached on disk and reused across builds whenever the same markup is requested at the same size.
