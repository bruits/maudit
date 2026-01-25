---
title: "Prefetching"
section: "core-concepts"
---

A downside of MPAs (Multi-Page Applications) contrary to SPAs (Single-Page Applications) is that navigating to other pages is often noticeably slower, as the browser has to load a totally new HTML document instead of just updating part of the current document.

Maudit has built-in support for prefetching pages before the user navigates to them, offering near-instant navigation, even for a MPA.

Prefetching is enabled by default and the default strategy is to prefetch pages on click down.

## Configuration

Prefetching can be configured using the `prefetch` property of [`BuildOptions`](https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html) which takes a [`PrefetchOptions`](https://docs.rs/maudit/latest/maudit/struct.PrefetchOptions.html) struct.

```rs
use maudit::{BuildOptions, PrefetchOptions, PrefetchStrategy};

BuildOptions {
  prefetch: PrefetchOptions {
    strategy: PrefetchStrategy::Hover,
    ..Default::default()
  },
  ..Default.default()
}
```

To disable prefetching, set `strategy` to [`PrefetchStrategy::None`](https://docs.rs/maudit/latest/maudit/enum.PrefetchStrategy.html#variant.None).

## Using the speculation rules API

Maudit will automatically uses the [Speculation Rules API](https://developer.mozilla.org/en-US/docs/Web/API/Speculation_Rules_API) to prefetch instead of `<link rel="prefetch">` tags when supported by the browser.

### Prerendering

By enabling `PrefetchOptions.prerender`, Maudit will also prerender your prefetched pages using the Speculation Rules API.

```rs
use maudit::{BuildOptions, PrefetchOptions, PrefetchStrategy};

BuildOptions {
  prefetch: PrefetchOptions {
    prerender: true,
    ..Default::default()
  },
  ..Default.default()
}
```

Note that prerendering, unlike prefetching, may require rethinking how the JavaScript on your pages works, as it'll run JavaScript from pages that the user hasn't visited yet. For example, this might result in analytics reporting incorrect page views.

## Possible risks 

Prefetching pages in static websites is typically always safe. In more traditional apps, an issue can arise if your pages cause side effects to happen on the server. For instance, if you were to prefetch `/logout`, your user might get disconnected on hover, or worse as soon as the log out link appear in the viewport. In modern times, it is typically not recommended to have links cause such side effects anyway, reducing the risk of this happening.

Additionally, the performance improvements provided by prefetching will, in the vast majority of cases, trump any possible resource wastage (of which the potential is low in the first place).
