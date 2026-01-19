---
title: "Prefetching"
section: "core-concepts"
---

A downside of MPAs (Multi-Page Applications) contrary to SPAs (Single-Page Applications) is that navigating to other pages is often noticeably slower, as the browser has to load a totally new HTML document instead of just updating part of the current document.

Maudit has built-in support for prefetching pages before the user navigates to them, offering near-instant navigation, even for a MPA.

Prefetching is enabled by default and the default strategy is to prefetch pages on click down.

## Configuration

Prefetching can be configured using the `prefetch` property of [`BuildOptions`](https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html) which takes a [`PrefetchOptions`](https://docs.rs/maudit/latest/maudit/struct.PrefetchOptions.html) struct. Currently, the only option is `strategy`.

```rs
use maudit::{BuildOptions, PrefetchOptions, PrefetchStrategy};

BuildOptions {
  prefetch: PrefetchOptions {
    strategy: PrefetchStrategy::Hover,
  },
  ..Default.default()
}
```

To disable prefetching, set `strategy` to [`PrefetchStrategy::None`](https://docs.rs/maudit/latest/maudit/enum.PrefetchStrategy.html#variant.None).
