---
cargo/maudit: minor
---

Adds OpenGraph image generation behind the new `og_image` feature. Call `ctx.assets.add_opengraph_image(source)` in a route with either inline SVG markup or an existing `Image`, then `og.render()` to emit the `<meta>` tags. Requires `BuildOptions::base_url` to be set.

```rust
let og = ctx.assets.add_opengraph_image(
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
          <rect width="100%" height="100%" fill="#1a1a1a"/>
          <text x="60" y="330" fill="white" font-size="72">Hello, world!</text>
        </svg>"##,
)?;

html! { head { (og.render()) } }
```
