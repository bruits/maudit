---
packages:
  - maudit
release: minor
---

Adds support for image processing. Maudit can now resize and convert images during the build process.

To process an image, add it using `ctx.assets.add_image_with_options` in your page's `render` method, specifying the desired transformations.

```rs
use maudit::page::prelude::*;

#[route("/image")]
pub struct ImagePage;

impl Page for ImagePage {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let image = ctx.assets.add_image_with_options(
      "path/to/image.jpg",
      ImageOptions {
        width: Some(800),
        height: None,
        format: Some(ImageFormat::Png),
        quality: Some(80),
      },
    )?;

    format!("<img src=\"{}\" alt=\"Processed Image\" />", image.url).into()
  }
}
```

See the [Assets documentation](https://maudit.org/docs/assets/) for more details.
