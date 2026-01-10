# maudit-macros

## 0.7.0 — 2026-01-10

### Minor changes

- [bce6fd1](https://github.com/bruits/maudit/commit/bce6fd12eb9c5275ea66c9205ad9b63889155ec2) Adds support for automatically generating sitemaps — Thanks @Princesseuh!

## 0.6.0 — 2026-01-04

### Minor changes

- [5a8a7de](https://github.com/bruits/maudit/commit/5a8a7de194de981dfb733d87cc5eb2d92b26deac) Fixes wrong version of maudit-macros being used — Thanks @Princesseuh!

## 0.5.0

### Minor changes

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


## 0.4.0

### Minor changes

- [52eda9e](https://github.com/bruits/maudit/commit/52eda9ea4eac8efd3efd945d00f39a1b99f284ab) Update generated code to support returning properties in dynamic routes. — Thanks @Princesseuh!

