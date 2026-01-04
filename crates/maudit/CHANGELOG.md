# maudit

## 0.8.0 — 2026-01-04

### Patch changes

- [5a8a7de](https://github.com/bruits/maudit/commit/5a8a7de194de981dfb733d87cc5eb2d92b26deac) Fixes wrong version of maudit-macros being used — Thanks @Princesseuh!
- Updated dependencies: maudit-macros (Cargo)@0.6.0

## 0.7.0 — 2026-01-03

### Minor changes

- [f40ccc7](https://github.com/bruits/maudit/commit/f40ccc75879b9af7788624959a74997ee47a016c) Adds support for generating variants of pages for the purpose of internationalization — Thanks @Princesseuh!

## 0.6.8 — 2025-12-12

### Patch changes

- [fc25ac2](https://github.com/bruits/maudit/commit/fc25ac2257ef7d38ff116b17418401fdb22f273c) Improve hashing performance — Thanks @Princesseuh!

## 0.6.7 — 2025-12-09

### Patch changes

- [9611a61](https://github.com/bruits/maudit/commit/9611a61e46b1950469ccd6518e37b486224f6cf4) Fixes cached entries for processed assets showing a longer log than intended — Thanks @Princesseuh!

## 0.6.6 — 2025-12-02

### Patch changes

- [1ce6701](https://github.com/bruits/maudit/commit/1ce6701f059a1a0ac6e62b4f0eadae2cfa26b4fe) Fixes Tailwind errors from stalling the build infinitely instead of erroring out — Thanks @Princesseuh!
- [696f653](https://github.com/bruits/maudit/commit/696f653a5f87d3271149b10d5022b49b56257653) Fixes logging of assets during build sometimes showing inconsistent paths — Thanks @Princesseuh!
- [696f653](https://github.com/bruits/maudit/commit/696f653a5f87d3271149b10d5022b49b56257653) Assets-related methods now all return Result, returning errors whenever files cannot be read or some other IO issue occurs. This makes it slightly more cumbersome to use, of course, however it makes it much easier to handle errors and return better error messages whenever something goes wrong. — Thanks @Princesseuh!
- [492953d](https://github.com/bruits/maudit/commit/492953d638939f86f3c933a9ed0febe4950f348a) Normalize the urls returned by `url()` to always properly reflect what the final path would look like — Thanks @Princesseuh!

## 0.6.5 — 2025-11-13

### Patch changes

- [1abd7ef](https://github.com/bruits/maudit/commit/1abd7ef239515a823c71a4be4dae858de0a44119) Adds support for customizing the location of the image cache — Thanks @Princesseuh!

## 0.6.4 — 2025-10-22

### Patch changes

- [5e9fc15](https://github.com/bruits/maudit/commit/5e9fc15fc1fa7a8f20f3ad261904235debf6d3db) Revert Tailwind hashing change, need a different solution. In the meantime, it is possible for stale content to appear with Tailwind in select conditions. — Thanks @Princesseuh!

## 0.6.3 — 2025-10-21

### Patch changes

- [371c3e3](https://github.com/bruits/maudit/commit/371c3e39202d5d28c156376ee38aceacbd1d10d3) Makes it so styles using Tailwind always have a different hash between builds in order to avoid stale content — Thanks @Princesseuh!
- [d69679a](https://github.com/bruits/maudit/commit/d69679aec307a7650ae203357e808cb62d5eff4e) Return a newtype around a String when using `Image.render()` so that images can be used directly in supported templating languages — Thanks @Princesseuh!

## 0.6.2

### Patch changes

- [0cb1738](https://github.com/bruits/maudit/commit/0cb173885de09e93d157308e809e07df7c57e7af) Updates MSRV to 1.89 — Thanks @Princesseuh!
- [36da26d](https://github.com/bruits/maudit/commit/36da26dfe5739b6d7077865289dfbfafea8ad60d) Add a new RenderWithAlt trait for rendering images, enforcing the usage of alt when trying to render to a String — Thanks @Princesseuh!


## 0.6.1

### Patch changes

- [c132d51](https://github.com/bruits/maudit/commit/c132d511d0038138a8bbc9b2122602a9154fa298) Assets' `url` method now always return `String` instead of `Option<String>` — Thanks @Princesseuh!
- [0113efe](https://github.com/bruits/maudit/commit/0113efe432936c4f4fd874e5ea0714cd3919974d) Add a new `base_url` setting and `canonical_url()` method on PageContext to make it easier to build absolute URLs inside pages — Thanks @Princesseuh!
- [c132d51](https://github.com/bruits/maudit/commit/c132d511d0038138a8bbc9b2122602a9154fa298) Fixed escaped shortcodes (i.e. `\{{ shortcode }}`) not rendering correctly — Thanks @Princesseuh!
- [c132d51](https://github.com/bruits/maudit/commit/c132d511d0038138a8bbc9b2122602a9154fa298) Refactored syntax highlighting into a `highlight_code` function that can be used independently of Markdown rendering — Thanks @Princesseuh!
- [c132d51](https://github.com/bruits/maudit/commit/c132d511d0038138a8bbc9b2122602a9154fa298) Improve performance when building many pages, especially when the pages are lightweight — Thanks @Princesseuh!
- [c132d51](https://github.com/bruits/maudit/commit/c132d511d0038138a8bbc9b2122602a9154fa298) Update Rolldown version — Thanks @Princesseuh!


## 0.6.0

### Minor changes

- [90cef9f](https://github.com/bruits/maudit/commit/90cef9f4049b8f2a236c622c564bfd29a4b6a8d2) Update the return type of `Route::render` to allow returning anything that can be converted into a `RenderResult`, such as `String` or `Result<String, E>`.
  
  This not only makes it more ergonomic to return strings directly from the `render` method, but also allows using the `?` operator to propagate errors without needing to change the function signature. This does require typing a few more characters, but it should be worth it for the improved ergonomics. Eventually, when https://github.com/rust-lang/rust/issues/63063 lands, it'll be hidden behind a simpler to write type alias. — Thanks @Princesseuh!
- [2bfa8a8](https://github.com/bruits/maudit/commit/2bfa8a87212243b27c2231b836e7da9ec2cd3288) Rename (almost) all instances of Routes to Pages and vice versa.
  
  Previously, in Maudit, a _page_ referred to the struct you'd pass to `coronate` and a page could have multiple routes if it was dynamic. In my opinion, the reverse is more intuitive: a _route_ is the struct you define, and a route can have multiple _pages_ if it's dynamic. This also applies to every other types that had "Route" or "Page" in their name.
  
  As such, the following renames were made:
  
  - `Route` -> `Page`
  - `FullRoute` -> `FullPage`
  - `RouteContext` -> `PageContext`
  - `RouteParams` -> `PageParams`
  - `Routes` -> `Pages`
  - `fn routes` -> `fn pages`
  - `maudit::page` -> `maudit::route` (including the prelude, which is now `maudit::route::prelude`)
  
  And probably some others I forgot. — Thanks @Princesseuh!

### Patch changes

- [4496b9b](https://github.com/bruits/maudit/commit/4496b9bcd8bbcdde7bd2d3b9b347aada6d182c0f) Improve hashing performance for assets — Thanks @Princesseuh!
- [4496b9b](https://github.com/bruits/maudit/commit/4496b9bcd8bbcdde7bd2d3b9b347aada6d182c0f) Changed syntax for self-closing shortcodes to require an explicit closing slash, ex: `{{ image /}}` — Thanks @Princesseuh!
- [4496b9b](https://github.com/bruits/maudit/commit/4496b9bcd8bbcdde7bd2d3b9b347aada6d182c0f) Adds width and height properties to images and generated html — Thanks @Princesseuh!
- Updated dependencies: maudit-macros@0.5.0


## 0.5.1

### Patch changes

- [9cd5fdd](https://github.com/bruits/maudit/commit/9cd5fdd8abe3044bd09d48b96217e3a0d2878b13) Updates default quality for webp to 80 to match sharp — Thanks @Princesseuh!

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
use maudit::route::prelude::*;

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

impl Route<Params, Props> for Post {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let params = ctx.params::<Params>();
    let props = ctx.props::<Props>();

    format!(
      "<h1>{}</h1><p>{}</p><small>Slug: {}</small>",
      props.title, props.content, params.slug
    ).into()
  }

  fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<Params, Props> {
    vec![Page::from_params_and_props(
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
