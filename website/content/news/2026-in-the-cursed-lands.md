---
title: "What's new in Maudit and what's coming in 2026"
description: "What is creeping around these cursed lands"
author: The Maudit Team
date: 2025-10-15
---

**How dusty are these shelves!** And yet, new books have found themselves nestled among the ancient tomes, ready to be discovered by those brave enough to explore these cursed lands.

Last summer we were very happy to introduce to you Maudit, a Rust library to generate static websites, then at version 0.1, but already quite mighty. This time we're back to talk about what we've been up to recently and what's coming this year.

> Interested in trying out Maudit? Follow our [Quick Start](/docs/quick-start/) guide.

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
