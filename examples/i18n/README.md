# i18n Example

This example demonstrates Maudit's **route variants** system using the `#[locales()]` macro for internationalization.

## Route Variants

Route variants allow a single route to have multiple versions with different paths. The `#[locales()]` macro is a convenient preset for defining locale-based variants.

## Examples

### Variant-Only Route

A route can exist _only_ as variants with no base path:

```rust
#[locales(en(path = "/en"), sv(path = "/sv"), de(path = "/de"))]
#[route]
pub struct Index;
```

This route has no base path - it only exists through its variants at `/en`, `/sv`, and `/de`.

### Route with Base Path and Variants

A route can have both a base path and localized variants:

```rust
#[locales(
    en(path = "/en/about"),
    sv(path = "/sv/om-oss"),
    de(path = "/de/uber-uns")
)]
#[route("/about")]
pub struct About;
```

This route is accessible at:

- `/about` - the base/default path
- `/en/about` - English variant
- `/sv/om-oss` - Swedish variant (using natural Swedish URL structure)
- `/de/uber-uns` - German variant (using natural German URL structure)

### Static Base with Dynamic Variants

**Yes, it's possible!** A route can have no base path (or a static base path) while having dynamic variants:

```rust
#[derive(Params, Clone)]
pub struct MixedParams {
    pub id: String,
}

// No base path, but dynamic variants
#[locales(en(path = "/en/products/[id]"), sv(path = "/sv/produkter/[id]"))]
#[route]
pub struct Mixed;

impl Route<MixedParams> for Mixed {
    fn pages(&self, _ctx: &mut DynamicRouteContext) -> Pages<MixedParams> {
        vec![
            Page::from_params(MixedParams { id: "laptop".to_string() }),
            Page::from_params(MixedParams { id: "phone".to_string() }),
        ]
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<MixedParams>();
        // Render using params.id
    }
}
```

This generates only variant pages:

- `/en/products/laptop` and `/en/products/phone`
- `/sv/produkter/laptop` and `/sv/produkter/phone`

The build system automatically detects that variants have dynamic parameters and generates all dynamic pages for each variant.

### Dynamic Routes with Variants

Dynamic routes can also have variants! Each variant will generate all the dynamic pages:

```rust
#[derive(Params, Clone)]
pub struct ArticleParams {
    pub slug: String,
}

#[locales(en(path = "/en/articles/[slug]"), sv(path = "/sv/artiklar/[slug]"))]
#[route("/articles/[slug]")]
pub struct Article;

impl Route<ArticleParams> for Article {
    fn pages(&self, _ctx: &mut DynamicRouteContext) -> Pages<ArticleParams> {
        vec![
            Page::from_params(ArticleParams { slug: "hello-world".to_string() }),
            Page::from_params(ArticleParams { slug: "getting-started".to_string() }),
        ]
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<ArticleParams>();
        // Render using params.slug
    }
}
```

This generates:

- `/articles/hello-world` and `/articles/getting-started` - base pages
- `/en/articles/hello-world` and `/en/articles/getting-started` - English variants
- `/sv/artiklar/hello-world` and `/sv/artiklar/getting-started` - Swedish variants (note the localized "artiklar" path segment)

## API

### Variant Metadata

Routes with variants expose a method via the `InternalRoute` trait:

- `variants(&self) -> Vec<(String, String)>` - Get all variants as `(id, path)` tuples

Example:

```rust
let about = About;
let variants = about.variants();
// Returns: vec![
//     ("en".to_string(), "/en/about".to_string()),
//     ("sv".to_string(), "/sv/om-oss".to_string()),
//     ("de".to_string(), "/de/uber-uns".to_string()),
// ]
```

### Variant Context

Both `DynamicRouteContext` and `PageContext` include variant information:

```rust
pub struct DynamicRouteContext<'a> {
    pub content: &'a mut ContentSources,
    pub assets: &'a mut RouteAssets,
    pub variant: Option<&'a str>, // None for base route, Some("en") for variants
}

pub struct PageContext<'a> {
    // ... other fields
    pub variant: Option<String>, // None for base route, Some("en") for variants
}
```

**Important:** `pages()` is called separately for each variant, so you can return different pages per variant:

```rust
impl Route<ArticleParams> for Article {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<ArticleParams> {
        match ctx.variant {
            Some("en") => {
                // Return English-only articles
                vec![Page::from_params(ArticleParams { slug: "hello".to_string() })]
            }
            Some("sv") => {
                // Return Swedish-only articles
                vec![Page::from_params(ArticleParams { slug: "hej".to_string() })]
            }
            None => {
                // Return all articles for base route
                vec![
                    Page::from_params(ArticleParams { slug: "hello".to_string() }),
                    Page::from_params(ArticleParams { slug: "hej".to_string() }),
                ]
            }
            _ => vec![]
        }
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<ArticleParams>();

        // Render differently based on variant
        let greeting = match ctx.variant.as_deref() {
            Some("en") => "Hello",
            Some("sv") => "Hej",
            None => "Hi",
            _ => "?"
        };

        format!("{}: {}", greeting, params.slug)
    }
}
```

## How It Works

When you run the build, Maudit automatically generates pages for all defined variants:

```
$ cargo run
generating pages
16:21:41 pages /en -> dist/en/index.html (+169μs)
16:21:41 pages /sv -> dist/sv/index.html (+51μs)
16:21:41 pages /de -> dist/de/index.html (+42μs)
16:21:41 pages /about -> dist/about/index.html (+56μs)
16:21:41 pages /en/about -> dist/en/about/index.html (+59μs)
16:21:41 pages /sv/om-oss -> dist/sv/om-oss/index.html (+54μs)
16:21:41 pages /de/uber-uns -> dist/de/uber-uns/index.html (+44μs)
16:38:08 build /articles/[slug]
16:38:08 pages ├─ dist/articles/hello-world/index.html (+90μs)
16:38:08 pages ├─ dist/articles/getting-started/index.html (+56μs)
16:38:08 pages ├─ dist/en/articles/hello-world/index.html (+85μs)
16:38:08 pages ├─ dist/en/articles/getting-started/index.html (+53μs)
16:38:08 pages ├─ dist/sv/artiklar/hello-world/index.html (+65μs)
16:38:08 pages ├─ dist/sv/artiklar/getting-started/index.html (+48μs)
16:38:08 pages generated 13 pages in 1ms
```

Each variant is treated as a separate page with its own URL and output file. For dynamic routes, all pages are generated for each variant automatically.
