use dyn_eq::DynEq;
use log::debug;
use rustc_hash::FxHashSet;
use std::hash::Hash;
use std::time::Instant;
use std::{fs, path::PathBuf};
use xxhash_rust::xxh3::xxh3_64;

mod image;
pub mod image_cache;
mod script;
mod style;
pub use image::{Image, ImageFormat, ImageOptions};
pub use script::Script;
pub use style::{Style, StyleOptions};

use crate::AssetHashingStrategy;
use crate::build::options::AssetsOptions;

#[derive(Default)]
pub struct PageAssets {
    pub images: FxHashSet<Image>,
    pub scripts: FxHashSet<Script>,
    pub styles: FxHashSet<Style>,

    pub(crate) options: PageAssetsOptions,
}

#[derive(Clone)]
pub struct PageAssetsOptions {
    pub assets_dir: PathBuf,
    pub hashing_strategy: AssetHashingStrategy,
}

impl Default for PageAssetsOptions {
    fn default() -> Self {
        let default_assets_options = AssetsOptions::default();
        Self {
            assets_dir: default_assets_options.assets_dir,
            hashing_strategy: default_assets_options.hashing_strategy,
        }
    }
}

impl PageAssets {
    pub fn new(assets_options: &PageAssetsOptions) -> Self {
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
    ///
    /// Subsequent calls to this function using the same path will return the same image, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_image_with_options<P>(&mut self, image_path: P, options: ImageOptions) -> Image
    where
        P: Into<PathBuf>,
    {
        let image_path = image_path.into();

        // Check if the image already exists in the assets, if so, return it
        if let Some(image) = self.images.iter().find_map(|asset| {
            asset.as_any().downcast_ref::<Image>().filter(|image| {
                image.path == image_path
                    && options == *image.options.as_ref().unwrap_or(&ImageOptions::default())
            })
        }) {
            return image.clone();
        }

        let image = Image {
            path: image_path.clone(),
            assets_dir: self.options.assets_dir.clone(),
            hash: calculate_hash(
                &image_path,
                Some(&HashConfig {
                    asset_type: HashAssetType::Image(&options),
                    hashing_strategy: &self.options.hashing_strategy,
                }),
            ),
            options: if options == ImageOptions::default() {
                None
            } else {
                Some(options)
            },
        };

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
    /// Alternatively, a script can be included automatically using the [PageAssets::include_script] method instead.
    ///
    /// Subsequent calls to this function using the same path will return the same script, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_script<P>(&mut self, script_path: P) -> Script
    where
        P: Into<PathBuf>,
    {
        let path = script_path.into();
        let script = Script {
            path: path.clone(),
            assets_dir: self.options.assets_dir.clone(),
            hash: calculate_hash(&path, None),
            included: false,
        };

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
        let path = script_path.into();
        let script = Script {
            path: path.clone(),
            assets_dir: self.options.assets_dir.clone(),
            hash: calculate_hash(&path, None),
            included: true,
        };

        self.scripts.insert(script);
    }

    /// Add a style to the page assets, causing the file to be created in the output directory. The style is resolved relative to the current working directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    /// Alternatively, a style can be included automatically using the [PageAssets::include_style] method instead.
    ///
    /// Subsequent calls to this method using the same path will return the same style, as such, the value returned by this method can be cloned and used multiple times without issue. this method is equivalent to calling `add_style_with_options` with the default `StyleOptions` and is purely provided for convenience.
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
        let path = style_path.into();
        let style = Style {
            path: path.clone(),
            assets_dir: self.options.assets_dir.clone(),
            hash: calculate_hash(
                &path,
                Some(&HashConfig {
                    asset_type: HashAssetType::Style(&options),
                    hashing_strategy: &self.options.hashing_strategy,
                }),
            ),
            tailwind: options.tailwind,
            included: false,
        };

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
        let path = style_path.into();
        let hash = calculate_hash(
            &path,
            Some(&HashConfig {
                asset_type: HashAssetType::Style(&options),
                hashing_strategy: &self.options.hashing_strategy,
            }),
        );
        let style = Style {
            path: path.clone(),
            assets_dir: self.options.assets_dir.clone(),
            hash,
            tailwind: options.tailwind,
            included: true,
        };

        self.styles.insert(style);
    }
}

#[allow(private_bounds)] // Users never interact with the internal trait, so it's fine
pub trait Asset: DynEq + InternalAsset + Sync + Send {
    fn build_path(&self) -> PathBuf {
        self.assets_dir().join(self.final_file_name())
    }
    fn url(&self) -> Option<String>;
    fn path(&self) -> &PathBuf;

    fn hash(&self) -> String;

    // TODO: I don't like these next two methods for scripts and styles, we should get this from Rolldown somehow, but I don't know how.
    // Our architecture is such that bundling runs after pages, so we can't know the final extension until then. We can't, and I don't want
    // to make it so we get assets beforehand because it'd make it less convenient and essentially cause us to act like a bundling framework.
    //
    // Perhaps it should be done as a post-processing step, like includes, but that'd require moving route finalization to after bundling,
    // which I'm not sure I want to do either. Plus, it'd be pretty slow if you have a layout on every page that includes a style/script (a fairly common case).
    //
    // An additional benefit would with that would also be to be able to avoid generating hashes for these files, but that's a smaller win.
    //
    // I don't know! - erika, 2025-09-01

    fn final_extension(&self) -> String {
        self.path()
            .extension()
            .map(|ext| ext.to_str().unwrap())
            .unwrap_or_default()
            .to_owned()
    }

    fn final_file_name(&self) -> String {
        let file_stem = self.path().file_stem().unwrap().to_str().unwrap();
        let extension = self.final_extension();

        if extension.is_empty() {
            format!("{}.{}", file_stem, self.hash())
        } else {
            format!("{}.{}.{}", file_stem, self.hash(), extension)
        }
    }
}

struct HashConfig<'a> {
    asset_type: HashAssetType<'a>,
    hashing_strategy: &'a AssetHashingStrategy,
}

enum HashAssetType<'a> {
    Image(&'a ImageOptions),
    Style(&'a StyleOptions),
}

fn calculate_hash(path: &PathBuf, options: Option<&HashConfig>) -> String {
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

trait InternalAsset {
    fn assets_dir(&self) -> &PathBuf;
}

impl Hash for dyn Asset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path().hash(state);
    }
}

dyn_eq::eq_trait_object!(Asset);

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
        let mut page_assets = PageAssets::default();
        page_assets.add_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
    }

    #[test]
    fn test_include_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        page_assets.include_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
        assert!(page_assets.included_styles().count() == 1);
    }

    #[test]
    fn test_add_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        page_assets.add_script(temp_dir.join("script.js"));
        assert!(page_assets.scripts.len() == 1);
    }

    #[test]
    fn test_include_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        page_assets.include_script(temp_dir.join("script.js"));

        assert!(page_assets.scripts.len() == 1);
        assert!(page_assets.included_scripts().count() == 1);
    }

    #[test]
    fn test_add_image() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        page_assets.add_image(temp_dir.join("image.png"));
        assert!(page_assets.images.len() == 1);
    }

    #[test]
    fn test_asset_has_leading_slash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        let image = page_assets.add_image(temp_dir.join("image.png"));
        assert_eq!(image.url().unwrap().chars().next(), Some('/'));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        assert_eq!(script.url().unwrap().chars().next(), Some('/'));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        assert_eq!(style.url().unwrap().chars().next(), Some('/'));
    }

    #[test]
    fn test_asset_url_include_hash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        let image = page_assets.add_image(temp_dir.join("image.png"));
        let image_hash = image.hash.clone();
        assert!(image.url().unwrap().contains(&image_hash));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        let script_hash = script.hash.clone();
        assert!(script.url().unwrap().contains(&script_hash));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        let style_hash = style.hash.clone();
        assert!(style.url().unwrap().contains(&style_hash));
    }

    #[test]
    fn test_asset_path_include_hash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets::default();

        let image = page_assets.add_image(temp_dir.join("image.png"));
        let image_hash = image.hash.clone();
        assert!(image.build_path().to_string_lossy().contains(&image_hash));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        let script_hash = script.hash.clone();
        assert!(script.build_path().to_string_lossy().contains(&script_hash));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        let style_hash = style.hash.clone();
        assert!(style.build_path().to_string_lossy().contains(&style_hash));
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

        let mut page_assets = PageAssets::default();

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

        let mut page_assets = PageAssets::default();

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

        let mut page_assets = PageAssets::new(&PageAssetsOptions::default());

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

        let mut page_assets = PageAssets::new(&PageAssetsOptions::default());

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

        let assets_options = PageAssetsOptions::default();
        let mut page_assets = PageAssets::new(&assets_options);

        // Write first content and get hash
        std::fs::write(&style_path, "body { background: red; }").unwrap();
        let style1 = page_assets.add_style(&style_path);
        let hash1 = style1.hash.clone();

        // Write different content and get new hash
        std::fs::write(&style_path, "body { background: green; }").unwrap();
        let style2 = Style {
            path: style_path.clone(),
            assets_dir: assets_options.assets_dir.clone(),
            hash: calculate_hash(
                &style_path,
                Some(&HashConfig {
                    asset_type: HashAssetType::Style(&StyleOptions::default()),
                    hashing_strategy: &AssetHashingStrategy::Precise,
                }),
            ),
            tailwind: false,
            included: false,
        };

        assert_ne!(
            hash1, style2.hash,
            "Different content should produce different hashes"
        );
    }
}
