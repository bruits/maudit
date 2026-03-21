# Documentation Review

An audit of the docs at `website/content/docs/` against the actual codebase.

---

## 1. Outdated version number in manual-install.md

**File:** `manual-install.md`
**Issue:** Suggests `maudit = "0.6"` and `maud = "0.27"`, but the current crate version is `0.11.1`.

```toml
# Doc says:
maudit = "0.6"

# Should be:
maudit = "0.11"
```

The `maud = "0.27"` version should also be verified against what's currently compatible.

---

## 2. Entrypoint doc inconsistency: `coronate` return type

**File:** `entrypoint.md`
**Issue:** The first example shows `coronate` returning `Result<BuildOutput, ...>`, but the second example (under "Registering Routes") shows `coronate` being called without handling the return value at all (`fn main() {`). This is inconsistent and the second example won't compile since `coronate` returns a `Result`.

```rs
// First example (correct):
fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
  coronate(routes![Index], content_sources![], BuildOptions::default())
}

// Second example (won't compile):
fn main() {
  coronate(
    routes![Index],
    content_sources![],
    BuildOptions::default()
  )
}
```

---

## 3. Content doc: `entry.data()` signature inconsistency

**File:** `content.md`
**Issue:** In the "Using a content source in pages" section, the code shows `entry.data(ctx)` which is correct (it takes a `ContentContext`). But in the "Custom loaders" section, the code shows `entry.data()` with no arguments:

```rs
let entry_data = entry.data(); // Doc shows this (incorrect)
```

The actual signature is `pub fn data<C: ContentContext>(&self, ctx: &mut C) -> &T`, so it always requires a context argument.

---

## 4. Content doc: `Entry::create` signature is wrong

**File:** `content.md`
**Issue:** The custom loader example shows:

```rs
vec![Entry::create(data.id.into(), None, None, data, vec![])]
```

But `Entry::create` is a trait method on `ContentEntry<T>`, not a direct method on `Entry`. The actual call path is `<Entry<MyType> as ContentEntry<MyType>>::create(...)` or more commonly the trait needs to be in scope. The docs don't mention that `ContentEntry` needs to be imported for this to work.

---

## 5. Prefetching doc: typo `Default.default()` instead of `Default::default()`

**File:** `prefetching.md`
**Issue:** Two instances of `..Default.default()` instead of `..Default::default()`:

```rs
// Doc says (line ~1017 and ~1039):
..Default.default()

// Should be:
..Default::default()
```

---

## 6. Prefetching doc: default strategy is wrong

**File:** `prefetching.md`
**Issue:** The doc says "the default strategy is to prefetch pages on click down" but the actual default in code is `PrefetchStrategy::Tap`, which is the same thing but uses different terminology. More importantly, the doc doesn't mention the `Viewport` strategy at all, which is available in the code.

Also, the `eagerness` option (`PrerenderEagerness` enum with `Immediate`, `Eager`, `Moderate`, `Conservative` variants) exists in the code but is not mentioned in the prefetching docs.

---

## 7. Styling doc: struct name mismatch in example

**File:** `styling.md`
**Issue:** The route struct is defined as `Index` but the `impl` block implements `Route for Blog`:

```rs
#[route("/")]
pub struct Index;

impl Route for Blog {  // Should be `impl Route for Index`
```

---

## 8. Templating doc: wrong API for adding assets

**File:** `templating.md`
**Issue:** The Maud example uses `ctx.add_style("style.css")` and `ctx.add_image("logo.png")`, but the actual API is `ctx.assets.add_style(...)` and `ctx.assets.add_image(...)`:

```rs
// Doc says:
let style = ctx.add_style("style.css");
let image = ctx.add_image("logo.png");

// Should be:
let style = ctx.assets.add_style("style.css")?;
let image = ctx.assets.add_image("logo.png")?;
```

---

## 9. Styling doc: `tailwind_binary_path` in wrong struct

**File:** `styling.md`
**Issue:** The docs reference `BuildOptions#tailwind_binary_path` but this field actually lives under `BuildOptions.assets.tailwind_binary_path` (in `AssetsOptions`). The default value is also wrong - the docs imply it defaults to a Node.js path, but the actual default is just `"tailwindcss"` (assuming a global install).

---

## 10. No documentation for pagination

**Issue:** The `paginate()` function, `PaginationPage<T>`, and `PaginatedContentPage<T>` are all exported through the prelude and are key features for building paginated routes. There's zero documentation about pagination in any of the docs pages. This is a significant feature gap.

The feature allows creating paginated dynamic routes like:
```rs
paginate(items, per_page, |page_num| MyParams { page: page_num })
```

---

## 11. No documentation for sitemap generation

**Issue:** `SitemapOptions` and `ChangeFreq` are publicly exported from the library and `sitemap` is a field on `BuildOptions`, but there's no documentation about sitemap generation anywhere. Users have no way to discover:
- `BuildOptions { sitemap: SitemapOptions { enabled: true, ... } }`
- Per-route sitemap metadata via `RouteSitemapMetadata`
- The `ChangeFreq` enum variants
- Stylesheet support for sitemaps

---

## 12. No documentation for `base_url` and `canonical_url()`

**Issue:** `BuildOptions::base_url` is available and `PageContext::canonical_url()` exists for generating canonical URLs, but neither is documented. The `base_url` option is only briefly mentioned in a code comment in the entrypoint doc but never explained.

---

## 13. No documentation for `props` system

**Issue:** Dynamic routes support a `Props` type parameter (`Route<Params, Props>`) and `ctx.props::<T>()` / `ctx.props_ref::<T>()` for passing data from `pages()` to `render()`. This is never documented but is used extensively with pagination.

---

## 14. No documentation for incremental builds

**Issue:** `BuildOptions::incremental` (defaults to `true`) enables incremental builds that only re-render pages whose dependencies changed. This is a significant performance feature that is completely undocumented.

---

## 15. No documentation for `into_pages()` / `into_params()` helpers

**Issue:** `TrackedContentSource` (returned by `ctx.content::<T>(...)`) has `into_pages()` and `into_params()` convenience methods that are the idiomatic way to generate dynamic routes from content. The content docs only show manual iteration patterns.

---

## 16. Content doc: `render_markdown` import path is wrong

**File:** `content.md`
**Issue:** The custom loader render example shows:

```rs
maudit::render_markdown(content, markdown_options, None, ctx)
```

But the actual function is at `maudit::content::markdown::render_markdown`, not at the crate root.

---

## 17. Content doc: shortcodes import path inconsistency

**File:** `content.md`
**Issue:** Shows `use maudit::shortcodes::MarkdownShortcodes` but the actual path is `maudit::content::markdown::shortcodes::MarkdownShortcodes`.

Similarly, shows `use maudit::components::MarkdownComponents` but the actual path is `maudit::content::markdown::components::MarkdownComponents`.

---

## 18. No documentation for `MarkdownHeading` and table of contents

**Issue:** `MarkdownContent::get_headings()` returns `Vec<MarkdownHeading>` (with `title`, `id`, `level`, `classes` fields) for building tables of contents. This is available on any `#[markdown_entry]` struct but not documented.

---

## 19. No documentation for `highlight_code` utility

**Issue:** `maudit::content::highlight_code` (and `HighlightOptions`, `CodeBlock`) are public APIs for syntax highlighting outside of Markdown. The content doc briefly mentions "Syntax highlighting can also be used outside of Markdown" but provides no usage example or API reference.

---

## 20. JavaScript doc: says "image file" instead of "script file"

**File:** `javascript.md`
**Issue:** The error handling description says "This function will return an error if the **image** file does not exist" when it should say "script file".

---

## 21. Quick Start links to wrong page for core concepts

**File:** `quick-start.md`
**Issue:** Says "please read the core concepts" and links to `/docs/content/` which is just one of the core concept pages, not an overview of all core concepts.

---

## 22. No documentation for `AssetHashingStrategy`

**Issue:** `AssetHashingStrategy` (`Precise` vs `FastImprecise`) is a publicly exported enum that affects build performance and caching behavior. The performance doc would be a natural place to mention this but it's absent.

---

## 23. Routing doc: `Route` trait missing `Params` generic on second dynamic route example

**File:** `routing.md`
**Issue:** The second dynamic route example shows `impl Route for Post` (line ~1236) without the `Params` generic, but it defines a `pages` method that requires `Route<Params>`. The first example correctly shows `impl Route<Params> for Post`.

---

## 24. Images doc: missing `ImageOptions` import

**File:** `images.md`
**Issue:** The image processing example uses `ImageOptions` and `ImageFormat` but doesn't show the import. These come from `maudit::assets` or the route prelude.

---

## Summary by severity

**Incorrect code that won't compile:**
- #2 (entrypoint missing Result return)
- #3 (entry.data() missing argument)
- #5 (Default.default() typo)
- #7 (struct name mismatch)
- #8 (wrong API path for assets)
- #16, #17 (wrong import paths)
- #23 (missing generic parameter)

**Outdated information:**
- #1 (version 0.6 vs 0.11)

**Missing feature documentation:**
- #10 (pagination)
- #11 (sitemaps)
- #12 (base_url / canonical_url)
- #13 (props system)
- #14 (incremental builds)
- #15 (into_pages/into_params)
- #18 (headings / TOC)
- #19 (highlight_code)
- #22 (asset hashing strategy)

**Minor issues:**
- #4 (ContentEntry import not mentioned)
- #6 (missing Viewport strategy, eagerness option)
- #9 (tailwind_binary_path location)
- #20 (image vs script typo)
- #21 (wrong link for core concepts)
- #24 (missing import)
