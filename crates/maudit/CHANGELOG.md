# maudit

## 0.4.0

### Minor changes

- [52eda9e](https://github.com/bruits/maudit/commit/52eda9ea4eac8efd3efd945d00f39a1b99f284ab) Adds support for image processing. Maudit can now resize and convert images during the build process.

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

See the [Assets documentation](https://maudit.org/docs/assets/) for more details. — Thanks @Princesseuh!
- [52eda9e](https://github.com/bruits/maudit/commit/52eda9ea4eac8efd3efd945d00f39a1b99f284ab) Adds support for dynamic routes with properties. In addition to its parameters, a dynamic route can now provide additional properties that can be used during rendering.

```rs
use maudit::page::prelude::*;

#[route("/posts/[slug]")]
pub struct Post;

#[derive(Params, Clone)]
pub struct Params {
  pub slug: String,
}

#[derive(Clone)]
pub struct Props {
  pub title: String,
  pub content: String,
}

impl Page<Params, Props> for Post {
  fn render(&self, ctx: &mut RouteContext) -> RenderResult {
    let params = ctx.params::<Params>();
    let props = ctx.props::<Props>();

    format!(
      "<h1>{}</h1><p>{}</p><small>Slug: {}</small>",
      props.title, props.content, params.slug
    ).into()
  }

  fn routes(&self, ctx: &mut DynamicRouteContext) -> Routes<Params, Props> {
    vec![Route::from_params_and_props(
      Params {
        slug: "hello-world".to_string(),
      },
      Props {
        title: "Hello World".to_string(),
        content: "This is my first post.".to_string(),
      },
    )]
  }
}
```

For more information on dynamic routes, see the [Routing documentation](https://maudit.org/docs/routing/#dynamic-routes). — Thanks @Princesseuh!

### Patch changes

- Updated dependencies: maudit-macros@0.4.0

