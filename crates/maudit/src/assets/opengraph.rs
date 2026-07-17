//! Generation of [OpenGraph](https://ogp.me/) images.
//!
//! SVG (an inline string or an `.svg` [`Image`]) is rendered to a PNG at build time using
//! [resvg](https://github.com/linebender/resvg); raster [`Image`]s are referenced as-is.
//! Obtain images through [`RouteAssets::add_opengraph_image`](crate::assets::RouteAssets::add_opengraph_image).

use std::fmt::Display;
use std::hash::Hasher;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use rapidhash::fast::RapidHasher;
use resvg::{tiny_skia, usvg};

use crate::assets::{Image, RouteAssets, make_filename, make_final_path, make_final_url};
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
        let mut tags = format!(r#"<meta property="og:image" content="{}"/>"#, self.url);
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
    /// Requires the `og_image` feature, which is enabled by default.
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
        // OpenGraph consumers require an absolute URL, so `base_url` must be set.
        let base_url = self.options.base_url.clone().ok_or_else(|| {
            AssetError::OpenGraphFailed {
                message: "OpenGraph images need an absolute URL: set `BuildOptions::base_url` to your site's URL (e.g. \"https://example.com\")".to_string(),
            }
        })?;

        match source.into() {
            OpenGraphSource::Svg(svg) => self.render_opengraph_svg(svg, &base_url),
            OpenGraphSource::Image(image) if is_svg(&image.path) => {
                let svg = std::fs::read_to_string(&image.path).map_err(|e| {
                    AssetError::OpenGraphFailed {
                        message: format!("failed to read {}: {}", image.path.display(), e),
                    }
                })?;
                self.render_opengraph_svg(&svg, &base_url)
            }
            OpenGraphSource::Image(image) => {
                // Raster images are valid OpenGraph formats, so reference the asset directly.
                self.images.insert(image.clone());
                // `dimensions()` reads the source, which no longer matches the output once
                // the image is resized; report unknown (0, 0) rather than wrong dimensions.
                let resized = image
                    .options
                    .as_ref()
                    .is_some_and(|opts| opts.width.is_some() || opts.height.is_some());
                let (width, height) = if resized { (0, 0) } else { image.dimensions() };
                Ok(OpenGraphImage {
                    url: format!("{}{}", base_url.trim_end_matches('/'), image.url),
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
        base_url: &str,
    ) -> Result<OpenGraphImage, AssetError> {
        let (png, width, height) = render_svg_to_png(svg)?;
        let hash = hash_bytes(&png);

        let filename = make_filename(Path::new("og-image"), &hash, Some("png"));
        let build_path = make_final_path(&self.options.output_assets_dir, &filename);
        let asset_url = make_final_url(&self.options.assets_dir, &filename);
        let url = format!("{}{}", base_url.trim_end_matches('/'), asset_url);

        // Materialize the generated PNG on disk so the build's copy step is a no-op.
        if !build_path.exists() {
            if let Some(parent) = build_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| AssetError::OpenGraphFailed {
                    message: format!("failed to create {}: {}", parent.display(), e),
                })?;
            }
            std::fs::write(&build_path, &png).map_err(|e| AssetError::OpenGraphFailed {
                message: format!("failed to write {}: {}", build_path.display(), e),
            })?;
        }

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

fn render_svg_to_png(svg: &str) -> Result<(Vec<u8>, u32, u32), AssetError> {
    let options = usvg::Options {
        fontdb: shared_fontdb(),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| AssetError::OpenGraphFailed {
        message: e.to_string(),
    })?;

    let size = tree.size();
    let width = (size.width().ceil() as u32).max(1);
    let height = (size.height().ceil() as u32).max(1);

    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| AssetError::OpenGraphFailed {
            message: format!("invalid image dimensions {width}x{height}"),
        })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let png = pixmap
        .encode_png()
        .map_err(|e| AssetError::OpenGraphFailed {
            message: e.to_string(),
        })?;

    Ok((png, width, height))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = RapidHasher::default();
    hasher.write(bytes);
    let hex = format!("{:016x}", hasher.finish());
    hex[..5].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
