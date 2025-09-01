use dyn_eq::DynEq;
use rustc_hash::FxHashSet;
use std::hash::Hash;
use std::path::Path;
use std::{fs, path::PathBuf};

#[derive(Default)]
pub struct PageAssets {
    pub(crate) assets: FxHashSet<Box<dyn Asset>>,
    pub(crate) scripts: FxHashSet<Script>,
    pub(crate) styles: FxHashSet<Style>,

    pub(crate) included_styles: Vec<Style>,
    pub(crate) included_scripts: Vec<Script>,

    pub(crate) assets_dir: PathBuf,
}

impl PageAssets {
    /// Add an image to the page assets, causing the file to be created in the output directory.
    ///
    /// The image will not automatically be included in the page, but can be included through the `.url()` method on the returned `Image` object.
    ///
    /// Subsequent calls to this function using the same path will return the same image, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_image<P>(&mut self, image_path: P) -> Image
    where
        P: Into<PathBuf>,
    {
        let image_path = image_path.into();

        // Check if the image already exists in the assets, if so, return it
        if let Some(image) = self.assets.iter().find_map(|asset| {
            asset
                .as_any()
                .downcast_ref::<Image>()
                .filter(|image| image.path == image_path)
        }) {
            return image.clone();
        }

        let image = Box::new(Image {
            path: image_path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&image_path),
        });

        self.assets.insert(image.clone());

        *image
    }

    /// Add a script to the page assets, causing the file to be created in the output directory.
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
            hash: calculate_hash(&path),
        };

        self.scripts.insert(script.clone());

        script
    }

    /// Include a script in the page
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
            hash: calculate_hash(&path),
        };

        self.scripts.insert(script.clone());
        self.included_scripts.push(script);
    }

    /// Add a style to the page assets, causing the file to be created in the output directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    ///
    /// Subsequent calls to this function using the same path will return the same style, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_style<P>(&mut self, style_path: P, options: Option<StyleOptions>) -> Style
    where
        P: Into<PathBuf>,
    {
        let path = style_path.into();
        let style = Style {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path),
            tailwind: options.as_ref().is_some_and(|opts| opts.tailwind),
        };

        self.styles.insert(style.clone());

        style
    }

    /// Include a style in the page
    ///
    /// This method will automatically include the style in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this function using the same path will result in the same style being included multiple times.
    pub fn include_style<P>(&mut self, style_path: P, options: Option<StyleOptions>)
    where
        P: Into<PathBuf>,
    {
        let path = style_path.into();
        let style = Style {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path),
            tailwind: options.as_ref().is_some_and(|opts| opts.tailwind),
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

    fn process(&self, _dist_assets_dir: &Path, _tmp_dir: &Path) -> Option<String> {
        None
    }

    fn hash(&self) -> String {
        // This will be overridden by each implementation to return the cached hash
        String::new()
    }

    fn final_file_name(&self) -> String {
        let file_stem = self.path().file_stem().unwrap().to_str().unwrap();
        let extension = self
            .path()
            .extension()
            .map(|ext| ext.to_str().unwrap())
            .unwrap_or("");

        if extension.is_empty() {
            format!("{}.{}", file_stem, self.hash())
        } else {
            format!("{}.{}.{}", file_stem, self.hash(), extension)
        }
    }
}

fn calculate_hash(path: &PathBuf) -> String {
    let content = fs::read(path).unwrap();

    let mut hasher = blake3::Hasher::new();
    hasher.update(&content);
    hasher.update(path.to_string_lossy().as_bytes());
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

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Image {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) hash: String,
}

impl InternalAsset for Image {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Image {
    fn url(&self) -> Option<String> {
        format!(
            "/{}/{}",
            self.assets_dir().to_string_lossy(),
            self.final_file_name()
        )
        .into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn hash(&self) -> String {
        self.hash.clone()
    }

    fn process(&self, dist_path: &Path, _: &Path) -> Option<String> {
        // TODO: Image processing
        fs::copy(&self.path, dist_path.join(self.final_file_name())).unwrap();

        None
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Script {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) hash: String,
}

impl InternalAsset for Script {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Script {
    fn url(&self) -> Option<String> {
        format!(
            "/{}/{}",
            self.assets_dir().to_string_lossy(),
            self.final_file_name()
        )
        .into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn hash(&self) -> String {
        self.hash.clone()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StyleOptions {
    pub tailwind: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Style {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) hash: String,
    pub(crate) tailwind: bool,
}

impl InternalAsset for Style {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Style {
    fn url(&self) -> Option<String> {
        format!(
            "/{}/{}",
            self.assets_dir().to_string_lossy(),
            self.final_file_name()
        )
        .into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn hash(&self) -> String {
        self.hash.clone()
    }

    fn process(&self, _: &Path, _: &Path) -> Option<String> {
        None
    }
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
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };
        page_assets.add_style(temp_dir.join("style.css"), None);

        assert!(page_assets.styles.len() == 1);
    }

    #[test]
    fn test_include_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.include_style(temp_dir.join("style.css"), None);

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
        assert!(page_assets.assets.len() == 1);
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

        let style = page_assets.add_style(temp_dir.join("style.css"), None);
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

        let style = page_assets.add_style(temp_dir.join("style.css"), None);
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

        let style = page_assets.add_style(temp_dir.join("style.css"), None);
        let style_hash = style.hash.clone();
        assert!(style.build_path().to_string_lossy().contains(&style_hash));
    }
}
