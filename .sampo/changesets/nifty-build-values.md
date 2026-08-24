---
cargo/maudit: minor
---

Adds `BuildValue`: a value computed once per build from content and shared across every page that reads it with `ctx.build_value(&MY_VALUE)`, with its content dependencies tracked so readers re-render when its inputs change.

Removes the unused `assets()` method from the `ContentContext` trait, and `PageContext::from_static_route` / `from_dynamic_route` now take a `&BuildValueStore` — both only affect code that drives the build loop directly instead of going through `coronate()`.
