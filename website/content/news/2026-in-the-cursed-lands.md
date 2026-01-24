---
title: "What's new in Maudit and what's coming in 2026"
description: "What is creeping around these cursed lands"
author: The Maudit Team
date: 2025-10-15
---

**How dusty are these shelves!** And yet, new books have found themselves nestled among the ancient tomes, ready to be discovered by those brave enough to explore these cursed lands.

Last summer we were very happy to introduce to you Maudit, a Rust library to generate static websites, then at version 0.1, but already quite mighty. This time we're back to talk about what we've been up to recently and what's coming this year.

> Interested in trying out Maudit? Follow our [Quick Start](/docs/quick-start/) guide.

## Image processing

[Maudit 0.4.0](https://github.com/bruits/maudit/blob/main/crates/maudit/CHANGELOG.md#040) added support for image processing, allowing you to easily resize, convert and optimize images for your website at build-time.

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

See [our section on image processing](https://maudit.org/docs/images/#processing-images) for more information on how to use images in Maudit.

### Placeholders generation

Maudit also includes the ability to easily create low-quality image placeholders (LQIP) for your images using [ThumbHash](https://evanw.github.io/thumbhash/).

```rs
use maudit::route::prelude::*;

#[route("/image")]
pub struct ImagePage;

impl Route for ImagePage {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let image = ctx.assets.add_image("path/to/image.jpg")?;
    let placeholder = image.placeholder();

    Ok(format!("<img src=\"{}\" alt=\"Image with placeholder\" style=\"background-image: url('{}'); background-size: cover;\" />", image.url(), placeholder.data_uri()))
  }
}
```

Check [our documentation on placeholders](/docs/images/#placeholders) for more information.

## Customizable Markdown rendering

[Maudit 0.5.0](https://github.com/bruits/maudit/blob/main/crates/maudit/CHANGELOG.md#050) added support for components and shortcodes in Markdown files. These features allows you to completely customize how your Markdown files are rendered and enhance them with cool new possibilities. 

### Shortcodes

Embedding a Youtube video typically requires one to copy this long, ugly, iframe tag and configure the different attributes to make sure it renders properly, it'd be nice to have something more friendly, a code that would be short, you will.

```md
Here's my cool video:

{{ youtube id="b_KfnGBtVeA" /}}
```

```rs
content_sources![
  "articles" => glob_markdown_with_options::<ArticleContent>("content/articles/*.md", MarkdownOptions {
    shortcodes: {
      let mut shortcodes = MarkdownShortcodes::default();

      shortcodes.register("youtube", |attrs, _| {
        if let Some(id) = attrs.get::<String>("id") {
          format!(r#"<iframe width="560" height="315" src="https://www.youtube.com/embed/{}" frameborder="0" allowfullscreen></iframe>"#, id)
        } else {
          panic!("YouTube shortcode requires an 'id' attribute");
        }
      });

      shortcodes
    },
  ..Default::default()
  })
],
```

For more information, read [our section on shortcodes](/docs/content/#shortcodes).

### Components

Sometimes, you want to be able to keep writing normal, spec-compliant, Markdown, but still be able to add a bit of spice to it. For this Maudit supports components, allowing you to use custom code when rendering normal Markdown elements. 

For instance, you may want to add an anchor icon to every heading, without needing to use a `{{ heading }}` shortcode.

```rs
use maudit::components::MarkdownComponents;

struct CustomHeading;

impl HeadingComponent for CustomHeading {
    fn render_start(&self, level: u8, id: Option<&str>, _classes: &[&str]) -> String {
        let id_attr = id.map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
        let href = id.map(|i| format!("#{}", i)).unwrap_or_default();
        format!(
            "<div><a href=\"{href}\"><span aria-hidden=\"true\">{}</span></a><h{level}{id_attr}>", include_str("icons/anchor.svg")
        )
    }

    fn render_end(&self, level: u8) -> String {
        format!("</h{level}></div>")
    }
}
```

```rs
content_sources![
    "blog" => glob_markdown_with_options::<BlogPost>("content/blog/**/*.md", MarkdownOptions {
      components: MarkdownComponents::new().heading(CustomHeading),
      ..Default::default()
    }),
],
```

For more information, read [our section on components](/docs/content/#components).

## Improved error handling

[Maudit 0.6.0](https://github.com/bruits/maudit/releases/tag/maudit-v0.6.0) and [0.6.6](https://github.com/bruits/maudit/releases/tag/maudit-v0.6.6) made it much easier to handle errors inside of pages by making all of the assets (which are quite prone to errors, filesystem and all) methods return Result instead of panicking. 

Additionally, pages themselves can now optionally return `Result` and will bubble up their errors up the chain up to [the entrypoint](/docs/entrypoint/) when using the `?` operator. Maudit implements `Into<RenderResult>` for `Result<T: Into<RenderResult>, E: Error>`, as such using `?` and returning `Result` require no signature changes inside your pages.

```rs
use maudit::route::prelude::*;
use maud::html;

#[route("/example")]
pub struct Example;

impl Route for Example {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        // Use the ? operator to bubble up asset-related errors
        let logo = ctx.assets.add_image("images/logo.png")?;

        // Wrap your return value with Ok()
        Ok(html! {
            (logo)
            p { "My cool logo!" }
        })
    }
}
```

Or, you can just `unwrap()` everything, that's ok! Check our section on [handling errors](/docs/routing/#handling-errors) if you'd like to learn more.

## Support for internationalization

[Maudit 0.7.0](https://github.com/bruits/maudit/releases/tag/maudit-v0.7.0) added support for internationalizating routes. For instance, you may want to have a `/about` in English, but `/a-propos` and `/om-oss` in French and Swedish respectively.

This is possible to do right now in Maudit: You can duplicate your `About` struct twice, register the two new routes, rewrite the `render` implementation twice.. but that's a bit cumbersome, so Maudit now allows you to generate all these pages using a single struct:

```rust
use maudit::route::prelude::*;

#[route(
    "/contact",
    locales(sv(prefix = "/sv"), de(path = "/de/kontakt"))
)]
pub struct Contact;

impl Route for Contact {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        match &ctx.variant {
            Some(language) => match language.as_str() {
                "sv" => "Kontakta oss.",
                "de" => "Kontaktieren Sie uns.",
                _ => unreachable!(),
            },
            _ => "Contact us.",
        }
    }
}
```

The ergonomics are still a bit iffy, but this nonetheless already makes it much easier to localize your website. To learn more about internationalization [visit our documentation](/docs/routing/#internationalization-i18n).

## Built-in sitemap generation

[Maudit 0.9.0](https://github.com/bruits/maudit/releases/tag/maudit-v0.9.0) added support for automatically generating a sitemap for your website. In this new world of AI and other advanced web crawlers, sitemaps are a bit of an old relic. However, they're still considered useful to ensure that search engines properly index your website.

To make Maudit generate a sitemap, first configure the [`base_url` property](https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html#structfield.base_url) on `BuildOptions` to your website's address and then enable sitemaps by setting [`sitemap`](https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html#structfield.sitemap) with a [SitemapOptions](https://docs.rs/maudit/latest/maudit/sitemap/struct.SitemapOptions.html) struct with `enabled: true`.

```rust
use maudit::{BuildOptions, SitemapOptions, content_sources, coronate, routes};

fn main() {
    coronate(
        routes![],
        content_sources![],
        BuildOptions {
            base_url: Some("https://example.com".into()),
            sitemap: SitemapOptions {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
}
```

With this, building your website will now result in a `sitemap.xml` file being generated inside your `dist` folder which includes all the pages of your website. Maudit will also automatically handle separating your sitemap in multiple files if you have over the recommended amount of maximum 50000 pages per sitemap.

For more information on sitemap generation in Maudit, check [our Sitemap documentation](/docs/sitemap/).

## Automatic prefetching

A common complaint about MPAs (Multi-Page Applications) is that navigating between page is slow, especially compared to the app-like experience of SPAs (Single-Page Applications).

The solution to this problem is to prefetch pages before the user navigate to them, like SPAs typically do, allowing near-instant navigations in most cases. 

Since [Maudit 0.10.0](https://github.com/bruits/maudit/releases/tag/maudit-v0.10.0), Maudit will by default prefetch links on clickdown, improving page loads by around 80ms on average, with other prefetching strategies available such as prefetching on hover.

<video autoplay controls loop>
  <source src="/prefetch.mp4" type="video/mp4">
</video>
<span class="text-center block italic">Showing the Hover strategy for prefetching</span>

For more information on prefetching, see [our prefetching documentation](/docs/prefetching/).

## Redirect utilities

[Maudit 0.10.0](https://github.com/bruits/maudit/releases/tag/maudit-v0.10.0) also added a new `redirect()` function to... well, redirect to another page.

```rust
use maudit::route::prelude::*;

#[route("/redirect")]
pub struct Redirect;

impl Route for Redirect {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        redirect("https://example.com")

        // Use a page's url method to generate type safe links:
        // redirect(&OtherPage.url(None))
    }
}
```

Simple enough. The return value of this function can be directly used in your pages, making it nice and easy to redirect to new content. To learn more about internationalization [redirect yourself to our documentation](/docs/routing/#redirects).

## The future

Maudit is mightier than before, but there's still so many twisted paths we'd like to follow. Including, but not limited to:

- Ability to generate variants of pages, outside of the localization system.
- Support for generating PWAs automatically
- Built-in font support (w/ subsetting)
- ... and more!

For now, we go back into hiding. See you soon!
