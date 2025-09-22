---
maudit: minor
---

Update the return type of `Route::render` to allow returning anything that can be converted into a `RenderResult`, such as `String` or `Result<String, E>`.

This not only makes it more ergonomic to return strings directly from the `render` method, but also allows using the `?` operator to propagate errors without needing to change the function signature. This does require typing a few more characters, but it should be worth it for the improved ergonomics. Eventually, when https://github.com/rust-lang/rust/issues/63063 lands, it'll be hidden behind a simpler to write type alias.
