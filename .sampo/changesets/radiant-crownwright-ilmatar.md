---
cargo/maudit: minor
---

Added built-in OpenGraph image generation behind the new `og_image` feature (enabled by default). Call `ctx.assets.add_opengraph_image(svg)` in a route to render an SVG string to a PNG at build time using [resvg](https://github.com/linebender/resvg). Like images, referencing the result is opt-in: `og.render()` returns the `<meta property="og:image">` tags.

```rust
let og = ctx.assets.add_opengraph_image(
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
          <rect width="100%" height="100%" fill="#1a1a1a"/>
          <text x="60" y="330" fill="white" font-size="72">Hello, world!</text>
        </svg>"##,
)?;

html! { head { (og.render()) } }
```
