---
cargo/maudit: minor
---

Added built-in OpenGraph image generation behind the new `og_image` feature (enabled by default). Call `ctx.assets.add_opengraph_image(source)` in a route with either an inline SVG string (rendered to a PNG at build time using [resvg](https://github.com/linebender/resvg), for dynamic per-page images) or an existing `Image` (an `.svg` is rendered to a PNG, raster images are referenced as-is, for static pre-made images). Like images, referencing the result is opt-in: `og.render()` returns the `<meta property="og:image">` tags. OpenGraph consumers require absolute image URLs, so `BuildOptions::base_url` must be set.

```rust
let og = ctx.assets.add_opengraph_image(
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
          <rect width="100%" height="100%" fill="#1a1a1a"/>
          <text x="60" y="330" fill="white" font-size="72">Hello, world!</text>
        </svg>"##,
)?;

html! { head { (og.render()) } }
```
