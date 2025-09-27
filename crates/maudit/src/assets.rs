use log::debug;
use rustc_hash::FxHashSet;
use std::path::Path;
use std::time::Instant;
use std::{fs, path::PathBuf};
use xxhash_rust::xxh3::xxh3_64;

mod image;
pub mod image_cache;
mod script;
mod style;
mod tailwind;
pub use image::{Image, ImageFormat, ImageOptions};
pub use script::Script;
pub use style::{Style, StyleOptions};
pub use tailwind::TailwindPlugin;

use crate::{AssetHashingStrategy, BuildOptions};

#[derive(Default)]
pub struct RouteAssets {
    pub images: FxHashSet<Image>,
    pub scripts: FxHashSet<Script>,
    pub styles: FxHashSet<Style>,

    pub(crate) options: RouteAssetsOptions,
}

#[derive(Clone)]
pub struct RouteAssetsOptions {
    pub assets_dir: PathBuf,
    pub output_assets_dir: PathBuf,
    pub hashing_strategy: AssetHashingStrategy,
}

impl Default for RouteAssetsOptions {
    fn default() -> Self {
        let default_build_options = BuildOptions::default();
        let page_assets_optiosn = default_build_options.route_assets_options();

        Self {
            assets_dir: default_build_options.assets.assets_dir,
            output_assets_dir: page_assets_optiosn.assets_dir,
            hashing_strategy: page_assets_optiosn.hashing_strategy,
        }
    }
}

impl RouteAssets {
    pub fn new(assets_options: &RouteAssetsOptions) -> Self {
        Self {
            options: assets_options.clone(),
            ..Default::default()
        }
    }

    pub fn assets(&self) -> impl Iterator<Item = &dyn Asset> {
        self.images
            .iter()
            .map(|asset| asset as &dyn Asset)
            .chain(self.scripts.iter().map(|asset| asset as &dyn Asset))
            .chain(self.styles.iter().map(|asset| asset as &dyn Asset))
    }

    /// Get all styles that are marked as included
    pub fn included_styles(&self) -> impl Iterator<Item = &Style> {
        self.styles.iter().filter(|s| s.included)
    }

    /// Get all scripts that are marked as included
    pub fn included_scripts(&self) -> impl Iterator<Item = &Script> {
        self.scripts.iter().filter(|s| s.included)
    }

    /// Add an image to the page assets, causing the file to be created in the output directory. The image is resolved relative to the current working directory.
    ///
    /// The image will not automatically be included in the page, but can be included through the `.url()` method on the returned `Image` object.
    pub fn add_image_with_options<P>(&mut self, image_path: P, options: ImageOptions) -> Image
    where
        P: Into<PathBuf>,
    {
        let image_path = image_path.into();

        let image_options = if options == ImageOptions::default() {
            None
        } else {
            Some(options)
        };

        let hash = calculate_hash(
            &image_path,
            Some(&HashConfig {
                asset_type: HashAssetType::Image(
                    image_options.as_ref().unwrap_or(&ImageOptions::default()),
                ),
                hashing_strategy: &self.options.hashing_strategy,
            }),
        );

        let image = Image::new(image_path, image_options, hash, &self.options);

        self.images.insert(image.clone());

        image
    }

    pub fn add_image<P>(&mut self, image_path: P) -> Image
    where
        P: Into<PathBuf>,
    {
        self.add_image_with_options(image_path, ImageOptions::default())
    }

    /// Add a script to the page assets, causing the file to be created in the output directory. The script is resolved relative to the current working directory.
    ///
    /// The script will not automatically be included in the page, but can be included through the `.url()` method on the returned `Script` object.
    /// Alternatively, a script can be included automatically using the [RouteAssets::include_script] method instead.
    pub fn add_script<P>(&mut self, script_path: P) -> Script
    where
        P: Into<PathBuf>,
    {
        let script_path = script_path.into();

        let hash = calculate_hash(
            &script_path,
            Some(&HashConfig {
                asset_type: HashAssetType::Script,
                hashing_strategy: &self.options.hashing_strategy,
            }),
        );

        let script = Script::new(script_path, false, hash, &self.options);

        self.scripts.insert(script.clone());

        script
    }

    /// Include a script in the page. The script is resolved relative to the current working directory.
    ///
    /// This method will automatically include the script in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this function using the same path will result in the same script being included multiple times.
    pub fn include_script<P>(&mut self, script_path: P)
    where
        P: Into<PathBuf>,
    {
        let script_path = script_path.into();

        let hash = calculate_hash(
            &script_path,
            Some(&HashConfig {
                asset_type: HashAssetType::Script,
                hashing_strategy: &self.options.hashing_strategy,
            }),
        );

        let script = Script::new(script_path, true, hash, &self.options);

        self.scripts.insert(script);
    }

    /// Add a style to the page assets, causing the file to be created in the output directory. The style is resolved relative to the current working directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    /// Alternatively, a style can be included automatically using the [RouteAssets::include_style] method instead.
    ///
    /// Subsequent calls to this method using the same path will return the same style, as such, the value returned by this method can be cloned and used multiple times without issue. This method is equivalent to calling `add_style_with_options` with the default `StyleOptions` and is purely provided for convenience.
    pub fn add_style<P>(&mut self, style_path: P) -> Style
    where
        P: Into<PathBuf>,
    {
        self.add_style_with_options(style_path, StyleOptions::default())
    }

    /// Add a style to the page assets, causing the file to be created in the output directory. The style is resolved relative to the current working directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    ///
    /// Subsequent calls to this method using the same path will return the same style, as such, the value returned by this method can be cloned and used multiple times without issue.
    pub fn add_style_with_options<P>(&mut self, style_path: P, options: StyleOptions) -> Style
    where
        P: Into<PathBuf>,
    {
        let style_path = style_path.into();

        let hash = calculate_hash(
            &style_path,
            Some(&HashConfig {
                asset_type: HashAssetType::Style(&options),
                hashing_strategy: &self.options.hashing_strategy,
            }),
        );

        let style = Style::new(style_path, false, &options, hash, &self.options);

        self.styles.insert(style.clone());

        style
    }

    /// Include a style in the page
    ///
    /// This method will automatically include the style in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this method using the same path will result in the same style being included multiple times. This method is equivalent to calling `include_style_with_options` with the default `StyleOptions` and is purely provided for convenience.
    pub fn include_style<P>(&mut self, style_path: P)
    where
        P: Into<PathBuf>,
    {
        self.include_style_with_options(style_path, StyleOptions::default())
    }

    /// Include a style in the page
    ///
    /// This method will automatically include the style in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this method using the same path will result in the same style being included multiple times.
    pub fn include_style_with_options<P>(&mut self, style_path: P, options: StyleOptions)
    where
        P: Into<PathBuf>,
    {
        let style_path = style_path.into();

        let hash = calculate_hash(
            &style_path,
            Some(&HashConfig {
                asset_type: HashAssetType::Style(&options),
                hashing_strategy: &self.options.hashing_strategy,
            }),
        );

        let style = Style::new(style_path, true, &options, hash, &self.options);

        self.styles.insert(style);
    }
}

pub trait Asset: Sync + Send {
    fn build_path(&self) -> &PathBuf;
    fn url(&self) -> &String;
    fn path(&self) -> &PathBuf;
    fn filename(&self) -> &PathBuf;
}

macro_rules! implement_asset_trait {
    ($type:ty) => {
        impl Asset for $type {
            fn path(&self) -> &PathBuf {
                &self.path
            }

            fn filename(&self) -> &PathBuf {
                &self.filename
            }

            fn build_path(&self) -> &PathBuf {
                &self.build_path
            }

            fn url(&self) -> &String {
                &self.url
            }
        }
    };
}

implement_asset_trait!(Image);
implement_asset_trait!(Script);
implement_asset_trait!(Style);

struct HashConfig<'a> {
    asset_type: HashAssetType<'a>,
    hashing_strategy: &'a AssetHashingStrategy,
}

enum HashAssetType<'a> {
    Image(&'a ImageOptions),
    Style(&'a StyleOptions),
    Script,
}

fn make_filename(path: &Path, hash: &String, extension: Option<&str>) -> PathBuf {
    let file_stem = path.file_stem().unwrap();

    let mut filename = PathBuf::new();
    filename.push(format!("{}.{}", file_stem.to_str().unwrap(), hash));

    if let Some(extension) = extension {
        filename.set_extension(format!("{}.{}", hash, extension));
    }

    filename
}

fn make_final_url(assets_dir: &Path, file_name: &Path) -> String {
    format!("/{}/{}", assets_dir.display(), file_name.display())
}

fn make_final_path(output_assets_dir: &Path, file_name: &Path) -> PathBuf {
    output_assets_dir.join(file_name)
}

fn calculate_hash(path: &Path, options: Option<&HashConfig>) -> String {
    let start_time = Instant::now();
    let content = if options
        .is_some_and(|cfg| *cfg.hashing_strategy == AssetHashingStrategy::FastImprecise)
    {
        let metadata = fs::metadata(path).unwrap();

        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(
            &metadata
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_le_bytes(),
        );

        buf.extend_from_slice(&metadata.len().to_le_bytes());

        buf
    } else {
        fs::read(path).unwrap_or_else(|_| panic!("Failed to read asset file: {:?}", path))
    };

    // Pre-allocate a single buffer to hash at once
    let mut buf = Vec::with_capacity(content.len() + 256);
    buf.extend_from_slice(&content);
    buf.extend_from_slice(path.to_string_lossy().as_bytes());

    if let Some(options) = options {
        match options.asset_type {
            HashAssetType::Image(opts) => {
                if let Some(width) = opts.width {
                    buf.extend_from_slice(&width.to_le_bytes());
                }
                if let Some(height) = opts.height {
                    buf.extend_from_slice(&height.to_le_bytes());
                }
                if let Some(format) = &opts.format {
                    buf.extend_from_slice(&format.to_hash_value().to_le_bytes());
                }
            }
            HashAssetType::Style(opts) => {
                buf.push(opts.tailwind as u8);
            }
            HashAssetType::Script => { /* No extra options for scripts yet */ }
        }
    }

    let hash = xxh3_64(&buf); // one-shot, much faster than streaming

    debug!(
        "Calculated hash for asset {:?} in {:?}",
        path,
        start_time.elapsed()
    );

    // TODO: This works, but perhaps we can generate prettier hashes, see https://github.com/rolldown/rolldown/blob/abf62c45d7a69b42dab4bff92095e320b418e9b8/crates/rolldown_utils/src/xxhash.rs
    let hex = format!("{:016x}", hash);
    hex[..5].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_temp_dir() -> PathBuf {
        // Create a temporary directory and test files
        let temp_dir = env::temp_dir().join("maudit_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("style.css"), "body { background: red; }").unwrap();
        std::fs::write(temp_dir.join("script.js"), "console.log('Hello, world!');").unwrap();
        std::fs::write(temp_dir.join("image.png"), b"").unwrap();
        temp_dir
    }

    #[test]
    fn test_add_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();
        page_assets.add_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
    }

    #[test]
    fn test_include_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        page_assets.include_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
        assert!(page_assets.included_styles().count() == 1);
    }

    #[test]
    fn test_add_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        page_assets.add_script(temp_dir.join("script.js"));
        assert!(page_assets.scripts.len() == 1);
    }

    #[test]
    fn test_include_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        page_assets.include_script(temp_dir.join("script.js"));

        assert!(page_assets.scripts.len() == 1);
        assert!(page_assets.included_scripts().count() == 1);
    }

    #[test]
    fn test_add_image() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        page_assets.add_image(temp_dir.join("image.png"));
        assert!(page_assets.images.len() == 1);
    }

    #[test]
    fn test_asset_has_leading_slash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        let image = page_assets.add_image(temp_dir.join("image.png"));
        assert_eq!(image.url().chars().next(), Some('/'));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        assert_eq!(script.url().chars().next(), Some('/'));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        assert_eq!(style.url().chars().next(), Some('/'));
    }

    #[test]
    fn test_asset_url_include_hash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        let image = page_assets.add_image(temp_dir.join("image.png"));
        assert!(image.url().contains(&image.hash));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        assert!(script.url().contains(&script.hash));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        assert!(style.url().contains(&style.hash));
    }

    #[test]
    fn test_asset_path_include_hash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = RouteAssets::default();

        let image = page_assets.add_image(temp_dir.join("image.png"));
        assert!(image.build_path().to_string_lossy().contains(&image.hash));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        assert!(script.build_path().to_string_lossy().contains(&script.hash));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        assert!(style.build_path().to_string_lossy().contains(&style.hash));
    }

    #[test]
    fn test_image_hash_different_options() {
        let temp_dir = setup_temp_dir();
        let image_path = temp_dir.join("image.png");

        // Create a simple test PNG (1x1 transparent pixel)
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&image_path, png_data).unwrap();

        let mut page_assets = RouteAssets::default();

        // Test that different options produce different hashes
        let image_default = page_assets.add_image(&image_path);
        let image_webp = page_assets.add_image_with_options(
            &image_path,
            ImageOptions {
                format: Some(ImageFormat::WebP),
                ..Default::default()
            },
        );
        let image_resized = page_assets.add_image_with_options(
            &image_path,
            ImageOptions {
                width: Some(100),
                height: Some(100),
                ..Default::default()
            },
        );
        let image_combined = page_assets.add_image_with_options(
            &image_path,
            ImageOptions {
                width: Some(100),
                height: Some(100),
                format: Some(ImageFormat::WebP),
            },
        );

        // All hashes should be different
        let hashes = [
            &image_default.hash,
            &image_webp.hash,
            &image_resized.hash,
            &image_combined.hash,
        ];

        for (i, hash1) in hashes.iter().enumerate() {
            for (j, hash2) in hashes.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        hash1, hash2,
                        "Hashes should be different for different options"
                    );
                }
            }
        }
    }

    #[test]
    fn test_image_hash_same_options() {
        let temp_dir = setup_temp_dir();
        let image_path = temp_dir.join("image.png");

        // Create a simple test PNG (1x1 transparent pixel)
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&image_path, png_data).unwrap();

        let mut page_assets = RouteAssets::default();

        // Same options should produce same hash
        let image1 = page_assets.add_image_with_options(
            &image_path,
            ImageOptions {
                width: Some(200),
                height: Some(150),
                format: Some(ImageFormat::Jpeg),
            },
        );

        let image2 = page_assets.add_image_with_options(
            &image_path,
            ImageOptions {
                width: Some(200),
                height: Some(150),
                format: Some(ImageFormat::Jpeg),
            },
        );

        assert_eq!(
            image1.hash, image2.hash,
            "Same options should produce same hash"
        );
    }

    #[test]
    fn test_style_hash_different_options() {
        let temp_dir = setup_temp_dir();
        let style_path = temp_dir.join("style.css");

        let mut page_assets = RouteAssets::new(&RouteAssetsOptions::default());

        // Test that different tailwind options produce different hashes
        let style_default = page_assets.add_style(&style_path);
        let style_tailwind =
            page_assets.add_style_with_options(&style_path, StyleOptions { tailwind: true });

        assert_ne!(
            style_default.hash, style_tailwind.hash,
            "Different tailwind options should produce different hashes"
        );
    }

    #[test]
    fn test_hash_includes_path() {
        let temp_dir = setup_temp_dir();

        // Create two identical files with different paths
        let content = "body { background: blue; }";
        let style1_path = temp_dir.join("style1.css");
        let style2_path = temp_dir.join("style2.css");

        std::fs::write(&style1_path, content).unwrap();
        std::fs::write(&style2_path, content).unwrap();

        let mut page_assets = RouteAssets::new(&RouteAssetsOptions::default());

        let style1 = page_assets.add_style(&style1_path);
        let style2 = page_assets.add_style(&style2_path);

        assert_ne!(
            style1.hash, style2.hash,
            "Different paths should produce different hashes even with same content"
        );
    }

    #[test]
    fn test_hash_includes_content() {
        let temp_dir = setup_temp_dir();
        let style_path = temp_dir.join("dynamic_style.css");

        let assets_options = RouteAssetsOptions::default();
        let mut page_assets = RouteAssets::new(&assets_options);

        // Write first content and get hash
        std::fs::write(&style_path, "body { background: red; }").unwrap();
        let style1 = page_assets.add_style(&style_path);
        let hash1 = style1.hash;

        // Write different content and get new hash
        std::fs::write(&style_path, "body { background: green; }").unwrap();
        let style2 = page_assets.add_style(&style_path);
        let hash2 = style2.hash;

        assert_ne!(
            hash1, hash2,
            "Different content should produce different hashes"
        );
    }
}
