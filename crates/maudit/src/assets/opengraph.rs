//! Generation of [OpenGraph](https://ogp.me/) images from SVG.
//!
//! SVG is rendered to a PNG at build time using [resvg](https://github.com/linebender/resvg).
//! Obtain images through [`RouteAssets::add_opengraph_image`](crate::assets::RouteAssets::add_opengraph_image).

use std::fmt::Display;
use std::hash::Hasher;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use rapidhash::fast::RapidHasher;
use resvg::{tiny_skia, usvg};

use crate::assets::{Image, RouteAssets, make_filename, make_final_path, make_final_url};
use crate::errors::AssetError;

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
}

impl OpenGraphImage {
    /// The URL of the generated image, e.g. to reference it manually.
    ///
    /// Absolute when [`BuildOptions::base_url`](crate::BuildOptions::base_url) is set (as OpenGraph
    /// consumers expect), otherwise root-relative.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The width of the generated image, in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height of the generated image, in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Render the `<meta>` tags needed to reference this image as the page's OpenGraph image.
    ///
    /// Like images, referencing the generated image in the page is opt-in: the image is
    /// created whether or not this method is called.
    pub fn render(&self) -> RenderedOpenGraphImage {
        format!(
            concat!(
                r#"<meta property="og:image" content="{url}"/>"#,
                r#"<meta property="og:image:type" content="image/png"/>"#,
                r#"<meta property="og:image:width" content="{width}"/>"#,
                r#"<meta property="og:image:height" content="{height}"/>"#,
            ),
            url = self.url,
            width = self.width,
            height = self.height,
        )
        .into()
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
    /// Generate an OpenGraph image from an SVG string, rendering it to a PNG at build time.
    ///
    /// The image is sized according to the SVG's own dimensions. As with [`add_image`](RouteAssets::add_image),
    /// the returned value can be referenced in the page through its [`url`](OpenGraphImage::url) or
    /// [`render`](OpenGraphImage::render) methods, but doing so is optional. The referenced URL is absolute
    /// when [`BuildOptions::base_url`](crate::BuildOptions::base_url) is set, as OpenGraph consumers expect.
    ///
    /// Requires the `og_image` feature, which is enabled by default.
    pub fn add_opengraph_image(&mut self, svg: &str) -> Result<OpenGraphImage, AssetError> {
        let (png, width, height) = render_svg_to_png(svg)?;
        let hash = hash_bytes(&png);

        let filename = make_filename(Path::new("og-image"), &hash, Some("png"));
        let build_path = make_final_path(&self.options.output_assets_dir, &filename);
        let asset_url = make_final_url(&self.options.assets_dir, &filename);

        // OpenGraph consumers expect an absolute URL, so resolve against `base_url` when set.
        let url = match &self.options.base_url {
            Some(base) => format!("{}{}", base.trim_end_matches('/'), asset_url),
            None => asset_url.clone(),
        };

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

        Ok(OpenGraphImage { url, width, height })
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
    use crate::assets::{Asset, RouteAssets, RouteAssetsOptions};

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630"><rect width="100%" height="100%" fill="#1a1a1a"/></svg>"##;

    fn assets_in(dir: &Path) -> RouteAssets {
        RouteAssets::new(
            &RouteAssetsOptions {
                output_assets_dir: dir.to_path_buf(),
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
    fn same_svg_produces_same_hash() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        let first = assets.add_opengraph_image(SVG).unwrap();
        let second = assets.add_opengraph_image(SVG).unwrap();

        assert_eq!(first.url(), second.url());
        assert_eq!(assets.images.len(), 1);
    }

    #[test]
    fn url_is_absolute_with_base_url() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = RouteAssets::new(
            &RouteAssetsOptions {
                output_assets_dir: temp_dir.path().to_path_buf(),
                base_url: Some("https://example.com".to_string()),
                ..Default::default()
            },
            None,
            None,
        );

        let og = assets.add_opengraph_image(SVG).unwrap();

        assert!(og.url().starts_with("https://example.com/"));
        assert!(og.render().to_string().contains(&format!(
            r#"<meta property="og:image" content="{}"/>"#,
            og.url()
        )));
    }

    #[test]
    fn invalid_svg_errors() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut assets = assets_in(temp_dir.path());

        assert!(assets.add_opengraph_image("not svg at all").is_err());
    }
}
