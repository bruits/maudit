# maudit

## 0.5.0

### Minor changes

- [d5a7fad](https://github.com/bruits/maudit/commit/d5a7fad563e9642be46b24d8db500e753c1175f5) The data URI and average RGBA for thumbnails is now calculated lazily, as such the `average_rgba` and `data_uri` fields have been replaced by methods. — Thanks @Princesseuh!
- [0403ac9](https://github.com/bruits/maudit/commit/0403ac9996f9d4e79945758fe06e7510729e383e) Add `is_dev()` function to allow one to toggle off things whenever running in dev — Thanks @Princesseuh!
- [39db004](https://github.com/bruits/maudit/commit/39db004b63ab7aa582a92593082e1261bae55b92) Added support for shortcodes in Markdown. Shortcodes allows you to substitute custom content in your Markdown files. This feature is useful for embedding dynamic content or reusable components within your Markdown documents.
  
  For instance, you might define a shortcode for embedding YouTube videos using only the video ID, or for inserting custom alerts or notes.
  
  ```markdown
  {{ youtube id="FbJ63spk48s" }}
  ```
  
  Would render to:
  
  ```html
  <iframe
    width="560"
    height="315"
    src="https://www.youtube.com/embed/FbJ63spk48s?si=hUGRndTWIThVY-72"
    title="YouTube video player"
    frameborder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerpolicy="strict-origin-when-cross-origin"
    allowfullscreen
  ></iframe>
  ```
  
  To define and register shortcodes, pass a MarkdownShortcodes instance to the MarkdownOptions when rendering Markdown content.
  
  ```rust
  let mut shortcodes = MarkdownShortcodes::new();
  
  shortcodes.register("youtube", |args, _ctx| {
      let id: String = args.get_required("id");
      format!(
          r#"<iframe width="560" height="315" src="https://www.youtube.com/embed/{}" frameborder="0" allowfullscreen></iframe>"#,
          id
      )
  });
  
  MarkdownOptions {
      shortcodes,
      ..Default::default()
  }
  
  // Then pass options to, i.e. glob_markdown in a content source
  ```
  
  Note that shortcodes are expanded before Markdown is rendered, so you can use shortcodes anywhere in your Markdown content, for instance in your frontmatter. Additionally, shortcodes may expand to Markdown content, which will then be rendered as part of the overall Markdown rendering process. — Thanks @Princesseuh!

### Patch changes

- [d5a7fad](https://github.com/bruits/maudit/commit/d5a7fad563e9642be46b24d8db500e753c1175f5) Added caching mechanism to placeholder and image transformation — Thanks @Princesseuh!


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

