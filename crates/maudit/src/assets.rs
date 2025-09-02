use dyn_eq::DynEq;
use rustc_hash::FxHashSet;
use std::hash::Hash;
use std::sync::OnceLock;
use std::{fs, path::PathBuf};

mod image;
mod script;
mod style;
pub use image::{Image, ImageFormat, ImageOptions};
pub use script::Script;
pub use style::{Style, StyleOptions};

#[derive(Default)]
pub struct PageAssets {
    pub(crate) images: FxHashSet<Image>,
    pub(crate) scripts: FxHashSet<Script>,
    pub(crate) styles: FxHashSet<Style>,

    pub(crate) included_styles: Vec<Style>,
    pub(crate) included_scripts: Vec<Script>,

    pub(crate) assets_dir: PathBuf,
}

impl PageAssets {
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
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&image_path, Some(HashConfig::Image(&options))),
            options: if options == ImageOptions::default() {
                None
            } else {
                Some(options)
            },
            __cache_placeholder: OnceLock::new(),
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
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path, None),
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
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path, None),
        };

        self.scripts.insert(script.clone());
        self.included_scripts.push(script);
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
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path, Some(HashConfig::Style(&options))),
            tailwind: options.tailwind,
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
        let hash = calculate_hash(&path, Some(HashConfig::Style(&options)));
        let style = Style {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash,
            tailwind: options.tailwind,
        };

        self.styles.insert(style.clone());
        self.included_styles.push(style);
    }
}

#[allow(private_bounds)] // Users never interact with the internal trait, so it's fine
pub trait Asset: DynEq + InternalAsset + Sync + Send {
    fn build_path(&self) -> PathBuf {
        self.assets_dir().join(self.final_file_name())
    }
    fn url(&self) -> Option<String>;
    fn path(&self) -> &PathBuf;

    fn hash(&self) -> String {
        // This will be overridden by each implementation to return the cached hash
        String::new()
    }

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

enum HashConfig<'a> {
    Image(&'a ImageOptions),
    Style(&'a StyleOptions),
}

fn calculate_hash(path: &PathBuf, options: Option<HashConfig>) -> String {
    let content = fs::read(path).unwrap();

    // TODO: Consider using xxhash for both performance and to match Rolldown's hashing
    let mut hasher = blake3::Hasher::new();
    hasher.update(&content);
    hasher.update(path.to_string_lossy().as_bytes());
    if let Some(options) = options {
        match options {
            HashConfig::Image(opts) => {
                let mut buf = Vec::new();
                if let Some(width) = opts.width {
                    buf.extend_from_slice(&width.to_le_bytes());
                }
                if let Some(height) = opts.height {
                    buf.extend_from_slice(&height.to_le_bytes());
                }
                if let Some(format) = &opts.format {
                    buf.extend_from_slice(&format.to_hash_value().to_le_bytes());
                }

                hasher.update(&buf);
            }
            HashConfig::Style(opts) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&[opts.tailwind as u8]);
                hasher.update(&buf);
            }
        }
    }
    let hash = hasher.finalize();

    // Take the first 5 characters of the hex string for a short hash like "al3hx"
    hash.to_hex()[..5].to_string()
}

trait InternalAsset {
    fn assets_dir(&self) -> PathBuf;
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
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };
        page_assets.add_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
    }

    #[test]
    fn test_include_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.include_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
        assert!(page_assets.included_styles.len() == 1);
    }

    #[test]
    fn test_add_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.add_script(temp_dir.join("script.js"));
        assert!(page_assets.scripts.len() == 1);
    }

    #[test]
    fn test_include_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.include_script(temp_dir.join("script.js"));

        assert!(page_assets.scripts.len() == 1);
        assert!(page_assets.included_scripts.len() == 1);
    }

    #[test]
    fn test_add_image() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.add_image(temp_dir.join("image.png"));
        assert!(page_assets.images.len() == 1);
    }

    #[test]
    fn test_asset_has_leading_slash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

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
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

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
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

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
}
