//! Generation of [OpenGraph](https://ogp.me/) images.
//!
//! SVG (an inline string or an `.svg` [`Image`]) is rendered to a PNG at build time using
//! [resvg](https://github.com/linebender/resvg); raster [`Image`]s are referenced as-is.
//! Obtain images through [`RouteAssets::add_opengraph_image`](crate::assets::RouteAssets::add_opengraph_image).
//!
//! Requires the `og_image` feature, which is not enabled by default.

use std::fmt::Display;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};

use crate::assets::{
    Image, RouteAssets, hash_bytes, join_base_url, make_filename, make_final_path, make_final_url,
};
use crate::errors::AssetError;

/// Source for [`RouteAssets::add_opengraph_image`].
///
/// A `&str`/`&String` (inline SVG) or an `&`[`Image`] can be passed directly through their
/// [`From`] implementations.
#[derive(Clone, Copy)]
pub enum OpenGraphSource<'a> {
    /// Inline SVG markup, rendered to a PNG. Best for images generated per-page.
    Svg(&'a str),
    /// An existing image asset. `.svg` files are rendered to a PNG; raster images (PNG,
    /// JPEG, WebP, …) are referenced as-is. Best for static, pre-made images.
    Image(&'a Image),
}

impl<'a> From<&'a str> for OpenGraphSource<'a> {
    fn from(svg: &'a str) -> Self {
        OpenGraphSource::Svg(svg)
    }
}

impl<'a> From<&'a String> for OpenGraphSource<'a> {
    fn from(svg: &'a String) -> Self {
        OpenGraphSource::Svg(svg)
    }
}

impl<'a> From<&'a Image> for OpenGraphSource<'a> {
    fn from(image: &'a Image) -> Self {
        OpenGraphSource::Image(image)
    }
}

// System fonts are expensive to enumerate, so the database is built once and
// shared across every image rendered during a build.
fn shared_fontdb() -> Arc<usvg::fontdb::Database> {
    static FONTDB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTDB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

/// A generated OpenGraph image asset, typically obtained using `ctx.assets.add_opengraph_image` in a route.
///
/// # Example
/// ```rust
/// use maudit::route::prelude::*;
///
/// #[route("/example")]
/// pub struct ExampleRoute;
///
/// impl Route for ExampleRoute {
///     fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
///         let og = ctx.assets.add_opengraph_image(
///             r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
///                  <rect width="100%" height="100%" fill="#1a1a1a"/>
///                  <text x="60" y="330" fill="white" font-size="72">Hello, world!</text>
///                </svg>"##,
///         )?;
///
///         Ok(format!("<head>{}</head>", og.render()))
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenGraphImage {
    url: String,
    width: u32,
    height: u32,
    content_type: Option<&'static str>,
}

impl OpenGraphImage {
    /// The absolute URL of the image, e.g. to reference it manually.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The width of the image in pixels, or `0` if unknown.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height of the image in pixels, or `0` if unknown.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Render the `<meta>` tags needed to reference this image as the page's OpenGraph image.
    ///
    /// Like images, referencing the image in the page is opt-in: the image is
    /// created whether or not this method is called.
    pub fn render(&self) -> RenderedOpenGraphImage {
        // The tags are emitted verbatim (the maud `Render` impl marks them pre-escaped), so the
        // URL — which embeds the user-provided `base_url` — must be HTML-attribute-escaped here.
        let mut tags = format!(
            r#"<meta property="og:image" content="{}"/>"#,
            escape_attribute(&self.url)
        );
        if let Some(content_type) = self.content_type {
            tags.push_str(&format!(
                r#"<meta property="og:image:type" content="{content_type}"/>"#
            ));
        }
        if self.width > 0 && self.height > 0 {
            tags.push_str(&format!(
                r#"<meta property="og:image:width" content="{}"/><meta property="og:image:height" content="{}"/>"#,
                self.width, self.height
            ));
        }
        tags.into()
    }
}

/// Newtype around a String representing the rendered OpenGraph `<meta>` tags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedOpenGraphImage(String);

impl From<String> for RenderedOpenGraphImage {
    fn from(value: String) -> Self {
        RenderedOpenGraphImage(value)
    }
}

impl Display for RenderedOpenGraphImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RouteAssets {
    /// Add an OpenGraph image, from either inline SVG (dynamic) or an existing [`Image`] (static).
    ///
    /// SVG — an [`OpenGraphSource::Svg`] string or an `.svg` [`Image`] — is rendered to a PNG at
    /// build time and sized according to the SVG's own dimensions. Raster [`Image`]s (PNG, JPEG,
    /// WebP, …) are referenced as-is. As with [`add_image`](RouteAssets::add_image), referencing
    /// the returned value in the page through its [`url`](OpenGraphImage::url) or
    /// [`render`](OpenGraphImage::render) methods is optional.
    ///
    /// OpenGraph consumers require absolute image URLs, so [`BuildOptions::base_url`](crate::BuildOptions::base_url)
    /// must be set; otherwise this returns an error.
    ///
    /// Requires the `og_image` feature, which is **not** enabled by default:
    /// `maudit = { version = "...", features = ["og_image"] }`.
    ///
    /// ## Example
    /// ```rust
    /// # use maudit::route::prelude::*;
    /// # fn example(ctx: &mut PageContext) -> Result<(), maudit::errors::AssetError> {
    /// // Dynamic: generated per page.
    /// let generated = ctx.assets.add_opengraph_image("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"630\"/>")?;
    ///
    /// // Static: a pre-made file.
    /// let cover = ctx.assets.add_image("images/og-cover.png")?;
    /// let static_og = ctx.assets.add_opengraph_image(&cover)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_opengraph_image<'a>(
        &mut self,
        source: impl Into<OpenGraphSource<'a>>,
    ) -> Result<OpenGraphImage, AssetError> {
        // OpenGraph consumers require an absolute URL, so `base_url` must be set. Cloned because
        // the SVG branches below borrow `&mut self`, which conflicts with holding a borrow of
        // `self.options.base_url` across the call.
        let base_url = self.options.base_url.clone().ok_or_else(|| {
            AssetError::OpenGraphFailed {
                message: "OpenGraph images need an absolute URL: set `BuildOptions::base_url` to your site's URL (e.g. \"https://example.com\")".to_string(),
            }
        })?;

        match source.into() {
            OpenGraphSource::Svg(svg) => self.render_opengraph_svg(svg, None, &base_url),
            OpenGraphSource::Image(image) if is_svg(&image.path) => {
                let svg = std::fs::read_to_string(&image.path).map_err(|e| {
                    AssetError::OpenGraphFailed {
                        message: format!("failed to read {}: {}", image.path.display(), e),
                    }
                })?;
                // Honor any resize requested via `add_image_with_options`, so the rendered PNG
                // (and its width/height meta tags) match the caller's chosen dimensions.
                let resize = image.options.as_ref().map(|opts| (opts.width, opts.height));
                self.render_opengraph_svg(&svg, resize, &base_url)
            }
            OpenGraphSource::Image(image) => {
                // Raster images are valid OpenGraph formats, so reference the asset directly.
                // `add_image` already registered it in `self.images`, so no insert is needed.
                // `dimensions()` reads the source, which no longer matches the output once
                // the image is resized; report unknown (0, 0) rather than wrong dimensions.
                let resized = image
                    .options
                    .as_ref()
                    .is_some_and(|opts| opts.width.is_some() || opts.height.is_some());
                let (width, height) = if resized { (0, 0) } else { image.dimensions() };
                Ok(OpenGraphImage {
                    url: join_base_url(&base_url, &image.url),
                    width,
                    height,
                    content_type: mime_from_url(&image.url),
                })
            }
        }
    }

    fn render_opengraph_svg(
        &mut self,
        svg: &str,
        resize: Option<(Option<u32>, Option<u32>)>,
        base_url: &str,
    ) -> Result<OpenGraphImage, AssetError> {
        // Key the cache on the *input* (SVG source + requested size), not the rendered bytes.
        // Rendering an SVG to a PNG is expensive (font-database load + rasterization), and this
        // runs on every page render, so reusing a previous render lets full rebuilds — which
        // re-run every route — skip the work entirely (the image cache is loaded independently
        // of the binary hash). An input-derived hash also yields a stable, deterministic URL,
        // unlike hashing the PNG bytes, which can drift across resvg/platform versions.
        let hash = hash_bytes(&opengraph_cache_key(svg, resize));

        let filename = make_filename(Path::new("og-image"), &hash, Some("png"));
        let build_path = make_final_path(&self.options.output_assets_dir, &filename);
        let asset_url = make_final_url(&self.options.assets_dir, &filename);
        let url = join_base_url(base_url, &asset_url);

        // Reuse a previously rendered PNG when the image cache still has one for this input.
        let cached = self
            .image_cache
            .as_ref()
            .and_then(|cache| cache.get_transformed_image(&filename));

        let (width, height) = if let Some(cache_path) = cached {
            // Cache hit: no rendering. Copy the cached PNG into the output dir if it isn't
            // already there, then recover its dimensions cheaply from the PNG header.
            materialize_from_cache(&cache_path, &build_path)?;
            image::image_dimensions(&build_path).unwrap_or((0, 0))
        } else {
            let (png, width, height) = render_svg_to_png(svg, resize)?;
            // Materialize the generated PNG on disk so the build's copy step is a no-op.
            write_generated(&build_path, &png)?;
            // Persist it to the image cache so subsequent builds can skip the render.
            if let Some(cache) = self.image_cache.as_ref() {
                let cache_path = cache.generate_cache_path(&filename);
                write_png(&cache_path, &png)?;
                cache.cache_transformed_image(&filename, cache_path);
            }
            (width, height)
        };

        self.images
            .insert(Image::from_generated(build_path, hash, filename, asset_url));

        Ok(OpenGraphImage {
            url,
            width,
            height,
            content_type: Some("image/png"),
        })
    }
}

/// Build the cache key bytes for a generated OpenGraph image: the SVG source plus the
/// requested resize, so the same SVG rendered at different sizes hashes differently.
fn opengraph_cache_key(svg: &str, resize: Option<(Option<u32>, Option<u32>)>) -> Vec<u8> {
    let mut key = Vec::with_capacity(svg.len() + 8);
    key.extend_from_slice(svg.as_bytes());
    let (w, h) = match resize {
        Some((w, h)) => (w.unwrap_or(0), h.unwrap_or(0)),
        None => (0, 0),
    };
    key.extend_from_slice(&w.to_le_bytes());
    key.extend_from_slice(&h.to_le_bytes());
    key
}

/// Materialize generated PNG `bytes` at `build_path`. Skipped when the file already exists —
/// the content-addressed name means any existing file has identical contents.
fn write_generated(build_path: &Path, bytes: &[u8]) -> Result<(), AssetError> {
    if build_path.exists() {
        return Ok(());
    }
    write_png(build_path, bytes)
}

/// Copy a cached PNG into the output directory when the output copy is missing.
fn materialize_from_cache(cache_path: &Path, build_path: &Path) -> Result<(), AssetError> {
    if build_path.exists() {
        return Ok(());
    }
    let bytes = std::fs::read(cache_path).map_err(|e| AssetError::OpenGraphFailed {
        message: format!("failed to read {}: {}", cache_path.display(), e),
    })?;
    write_generated(build_path, &bytes)
}

fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

fn mime_from_url(url: &str) -> Option<&'static str> {
    match url.rsplit('.').next()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}

/// HTML-attribute-escape a string destined for a double-quoted attribute value.
fn escape_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Write PNG `bytes` to `path`, creating parent directories as needed.
fn write_png(path: &Path, bytes: &[u8]) -> Result<(), AssetError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AssetError::OpenGraphFailed {
            message: format!("failed to create {}: {}", parent.display(), e),
        })?;
    }
    std::fs::write(path, bytes).map_err(|e| AssetError::OpenGraphFailed {
        message: format!("failed to write {}: {}", path.display(), e),
    })
}

fn render_svg_to_png(
    svg: &str,
    resize: Option<(Option<u32>, Option<u32>)>,
) -> Result<(Vec<u8>, u32, u32), AssetError> {
    let options = usvg::Options {
        fontdb: shared_fontdb(),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| AssetError::OpenGraphFailed {
        message: e.to_string(),
    })?;

    let size = tree.size();
    let intrinsic_width = size.width();
    let intrinsic_height = size.height();

    // A requested width/height scales the SVG to fit within the box while preserving its
    // aspect ratio, matching how raster `ImageOptions` resizing behaves.
    let scale = match resize {
        Some((Some(w), Some(h))) => (w as f32 / intrinsic_width).min(h as f32 / intrinsic_height),
        Some((Some(w), None)) => w as f32 / intrinsic_width,
        Some((None, Some(h))) => h as f32 / intrinsic_height,
        _ => 1.0,
    };

    let width = ((intrinsic_width * scale).ceil() as u32).max(1);
    let height = ((intrinsic_height * scale).ceil() as u32).max(1);

    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| AssetError::OpenGraphFailed {
            message: format!("invalid image dimensions {width}x{height}"),
        })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let png = pixmap
        .encode_png()
        .map_err(|e| AssetError::OpenGraphFailed {
            message: e.to_string(),
        })?;

    Ok((png, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::image_cache::ImageCache;
    use crate::assets::{Asset, ImageOptions, RouteAssets, RouteAssetsOptions};

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630"><rect width="100%" height="100%" fill="#1a1a1a"/></svg>"##;

    fn assets_in(dir: &Path) -> RouteAssets {
        RouteAssets::new(
            &RouteAssetsOptions {
                output_assets_dir: dir.to_path_buf(),
                base_url: Some("https://example.com".to_string()),
                ..Default::default()
            },
            None,
            None,
        )
    }

    fn assets_with_cache(output_dir: &Path, cache: ImageCache) -> RouteAssets {
        RouteAssets::new(
            &RouteAssetsOptions {
                output_assets_dir: output_dir.to_path_buf(),
                base_url: Some("https://example.com".to_string()),
                ..Default::default()
            },
            Some(cache),
            None,
        )
    }

    #[test]
    fn generates_image_asset() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        let og = assets.add_opengraph_image(SVG).unwrap();

        assert_eq!((og.width(), og.height()), (1200, 630));
        assert_eq!(assets.images.len(), 1);
        assert!(og.url().ends_with(".png"));

        let image = assets.images.iter().next().unwrap();
        assert!(image.build_path().exists());
        assert!(
            std::fs::read(image.build_path())
                .unwrap()
                .starts_with(b"\x89PNG")
        );
    }

    #[test]
    fn render_emits_meta_tags() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        let og = assets.add_opengraph_image(SVG).unwrap();
        let rendered = og.render().to_string();

        assert!(rendered.contains(&format!(
            r#"<meta property="og:image" content="{}"/>"#,
            og.url()
        )));
        assert!(rendered.contains(r#"<meta property="og:image:width" content="1200"/>"#));
        assert!(rendered.contains(r#"<meta property="og:image:height" content="630"/>"#));
    }

    #[test]
    fn references_raster_image_directly() {
        let temp_dir = tempfile::tempdir().unwrap();
        let img_path = temp_dir.path().join("cover.png");
        image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(8, 4, |_, _| {
            image::Rgba([10, 10, 10, 255])
        })
        .save(&img_path)
        .unwrap();

        let mut assets = assets_in(temp_dir.path());
        let image = assets.add_image(&img_path).unwrap();
        let og = assets.add_opengraph_image(&image).unwrap();

        assert_eq!((og.width(), og.height()), (8, 4));
        assert!(og.url().starts_with("https://example.com/"));
        assert!(og.url().ends_with(".png"));
        // No PNG is generated; only the referenced image is registered.
        assert_eq!(assets.images.len(), 1);

        let rendered = og.render().to_string();
        assert!(rendered.contains(r#"<meta property="og:image:type" content="image/png"/>"#));
        assert!(rendered.contains(r#"<meta property="og:image:width" content="8"/>"#));
    }

    #[test]
    fn resized_raster_image_omits_dimensions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let img_path = temp_dir.path().join("cover.png");
        image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(20, 20, |_, _| {
            image::Rgba([10, 10, 10, 255])
        })
        .save(&img_path)
        .unwrap();

        let mut assets = assets_in(temp_dir.path());
        let image = assets
            .add_image_with_options(
                &img_path,
                ImageOptions {
                    width: Some(8),
                    height: Some(8),
                    format: None,
                },
            )
            .unwrap();
        let og = assets.add_opengraph_image(&image).unwrap();

        assert_eq!((og.width(), og.height()), (0, 0));
        let rendered = og.render().to_string();
        assert!(!rendered.contains("og:image:width"));
        assert!(rendered.contains(r#"<meta property="og:image:type""#));
    }

    #[test]
    fn renders_svg_image_to_png() {
        let temp_dir = tempfile::tempdir().unwrap();
        let svg_path = temp_dir.path().join("cover.svg");
        std::fs::write(&svg_path, SVG).unwrap();

        let mut assets = assets_in(temp_dir.path());
        let image = assets.add_image(&svg_path).unwrap();
        let og = assets.add_opengraph_image(&image).unwrap();

        assert_eq!((og.width(), og.height()), (1200, 630));
        assert!(og.url().ends_with(".png"));
        // The source SVG plus the generated PNG.
        assert_eq!(assets.images.len(), 2);
    }

    #[test]
    fn same_svg_produces_same_hash() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        let first = assets.add_opengraph_image(SVG).unwrap();
        let second = assets.add_opengraph_image(SVG).unwrap();

        assert_eq!(first.url(), second.url());
        assert_eq!(assets.images.len(), 1);
    }

    #[test]
    fn url_is_absolute() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        let og = assets.add_opengraph_image(SVG).unwrap();

        assert!(og.url().starts_with("https://example.com/"));
    }

    #[test]
    fn errors_without_base_url() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = RouteAssets::new(
            &RouteAssetsOptions {
                output_assets_dir: temp_dir.path().to_path_buf(),
                base_url: None,
                ..Default::default()
            },
            None,
            None,
        );

        assert!(assets.add_opengraph_image(SVG).is_err());
    }

    #[test]
    fn invalid_svg_errors() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        assert!(assets.add_opengraph_image("not svg at all").is_err());
    }

    #[test]
    fn render_escapes_url_special_characters() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = RouteAssets::new(
            &RouteAssetsOptions {
                output_assets_dir: temp_dir.path().to_path_buf(),
                // A base_url carrying query-string characters must not break the meta tag.
                base_url: Some("https://example.com/?a=1&b=2".to_string()),
                ..Default::default()
            },
            None,
            None,
        );

        let og = assets.add_opengraph_image(SVG).unwrap();
        let rendered = og.render().to_string();

        // url() returns the raw URL for programmatic use; render() escapes it for HTML.
        assert!(og.url().contains("&b=2"));
        assert!(rendered.contains("&amp;b=2"));
        assert!(!rendered.contains("&b=2"));
    }

    #[test]
    fn svg_image_honors_resize_options() {
        let temp_dir = tempfile::tempdir().unwrap();
        let svg_path = temp_dir.path().join("cover.svg");
        std::fs::write(&svg_path, SVG).unwrap();

        let mut assets = assets_in(temp_dir.path());
        // Intrinsic SVG is 1200x630; requesting width 600 fits within while preserving aspect.
        let image = assets
            .add_image_with_options(
                &svg_path,
                ImageOptions {
                    width: Some(600),
                    height: None,
                    format: None,
                },
            )
            .unwrap();
        let og = assets.add_opengraph_image(&image).unwrap();

        assert_eq!((og.width(), og.height()), (600, 315));
        let rendered = og.render().to_string();
        assert!(rendered.contains(r#"<meta property="og:image:width" content="600"/>"#));
        assert!(rendered.contains(r#"<meta property="og:image:height" content="315"/>"#));
    }

    #[test]
    fn caches_render_and_reuses_across_builds() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let cache = ImageCache::with_cache_dir(&cache_dir);

        // First build: renders the SVG and stores the PNG in the image cache.
        let out1 = temp_dir.path().join("out1");
        let mut assets1 = assets_with_cache(&out1, cache.clone());
        let og1 = assets1.add_opengraph_image(SVG).unwrap();
        assert_eq!((og1.width(), og1.height()), (1200, 630));

        // Find the cached PNG and overwrite it with a distinctly-sized image. A second render
        // of the same SVG would still produce 1200x630, so if the cache is consulted instead
        // the reported dimensions will match this stand-in — proving the render was skipped.
        let cached_png = std::fs::read_dir(&cache_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().is_some_and(|ext| ext == "png"))
            .expect("cache should hold a rendered PNG")
            .path();
        image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(8, 4, |_, _| image::Rgba([1, 2, 3, 255]))
            .save(&cached_png)
            .unwrap();

        // Second build into a fresh output dir: must reuse the (overwritten) cached PNG.
        let out2 = temp_dir.path().join("out2");
        let mut assets2 = assets_with_cache(&out2, cache);
        let og2 = assets2.add_opengraph_image(SVG).unwrap();

        assert_eq!(og1.url(), og2.url());
        assert_eq!(
            (og2.width(), og2.height()),
            (8, 4),
            "second build should reuse the cached render, not re-render the SVG"
        );

        // The reused PNG is materialized into the new output directory.
        let materialized = assets2.images.iter().next().unwrap();
        assert!(materialized.build_path().exists());
        assert!(materialized.build_path().starts_with(&out2));
    }

    #[test]
    fn accepts_string_svg_source() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        // Exercises the `From<&String>` conversion (distinct from the `&str` one).
        let svg = String::from(SVG);
        let og = assets.add_opengraph_image(&svg).unwrap();

        assert_eq!((og.width(), og.height()), (1200, 630));
    }

    #[test]
    fn different_svg_produces_different_url() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        let a = assets.add_opengraph_image(SVG).unwrap();
        let b = assets
            .add_opengraph_image(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="418"/>"##,
            )
            .unwrap();

        assert_ne!(a.url(), b.url());
        assert_eq!(assets.images.len(), 2);
    }

    #[test]
    fn escape_attribute_escapes_html_metacharacters() {
        assert_eq!(escape_attribute("a&b"), "a&amp;b");
        assert_eq!(escape_attribute(r#"a"b"#), "a&quot;b");
        assert_eq!(escape_attribute("a<b>c"), "a&lt;b&gt;c");
        assert_eq!(escape_attribute("nothing-special"), "nothing-special");
    }

    #[test]
    fn mime_from_url_covers_known_extensions() {
        assert_eq!(mime_from_url("/a/x.png"), Some("image/png"));
        assert_eq!(mime_from_url("/a/x.jpg"), Some("image/jpeg"));
        assert_eq!(mime_from_url("/a/x.jpeg"), Some("image/jpeg"));
        assert_eq!(mime_from_url("/a/x.webp"), Some("image/webp"));
        assert_eq!(mime_from_url("/a/x.gif"), Some("image/gif"));
        assert_eq!(mime_from_url("/a/x.avif"), Some("image/avif"));
        // Unknown / missing extensions yield no `og:image:type`.
        assert_eq!(mime_from_url("/a/x.svg"), None);
        assert_eq!(mime_from_url("/a/noext"), None);
    }

    #[test]
    fn cache_key_distinguishes_source_and_size() {
        let base = opengraph_cache_key(SVG, None);

        // Same input → same key (deterministic).
        assert_eq!(base, opengraph_cache_key(SVG, None));
        // Different SVG → different key.
        assert_ne!(base, opengraph_cache_key("<svg/>", None));
        // Same SVG, different requested sizes → different keys.
        let w600 = opengraph_cache_key(SVG, Some((Some(600), None)));
        let w300 = opengraph_cache_key(SVG, Some((Some(300), None)));
        assert_ne!(base, w600);
        assert_ne!(w600, w300);
    }

    #[test]
    fn render_svg_to_png_resize_variants() {
        // The constant SVG is intrinsically 1200x630 (aspect ratio preserved on resize).
        let dims = |resize| {
            let (_, w, h) = render_svg_to_png(SVG, resize).unwrap();
            (w, h)
        };

        assert_eq!(dims(None), (1200, 630));
        assert_eq!(dims(Some((Some(600), None))), (600, 315)); // width-only
        assert_eq!(dims(Some((None, Some(315)))), (600, 315)); // height-only
        assert_eq!(dims(Some((Some(600), Some(600)))), (600, 315)); // both → min scale
    }
}
