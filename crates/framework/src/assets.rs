use dyn_eq::DynEq;
use log::info;
use rustc_hash::FxHashSet;
use std::hash::Hash;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;
use std::{fs, path::PathBuf};

#[derive(Default)]
pub struct PageAssets {
    pub(crate) assets: FxHashSet<Box<dyn Asset>>,
    pub(crate) scripts: FxHashSet<Script>,
    pub(crate) styles: FxHashSet<Style>,

    pub(crate) included_styles: Vec<Style>,
    pub(crate) included_scripts: Vec<Script>,

    pub(crate) assets_dir: PathBuf,
    pub(crate) tailwind_path: PathBuf,
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
            path: image_path,
            assets_dir: self.assets_dir.clone(),
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
        let script = Script {
            path: script_path.into(),
            assets_dir: self.assets_dir.clone(),
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
        let script = Script {
            path: script_path.into(),
            assets_dir: self.assets_dir.clone(),
        };

        self.scripts.insert(script.clone());
        self.included_scripts.push(script);
    }

    /// Add a style to the page assets, causing the file to be created in the output directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    ///
    /// Subsequent calls to this function using the same path will return the same style, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_style<P>(&mut self, style_path: P, tailwind: bool) -> Style
    where
        P: Into<PathBuf>,
    {
        let style = Style {
            path: style_path.into(),
            tailwind,
            assets_dir: self.assets_dir.clone(),
            tailwind_path: self.tailwind_path.clone(),
        };

        self.styles.insert(style.clone());

        style
    }

    /// Include a style in the page
    ///
    /// This method will automatically include the style in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this function using the same path will result in the same style being included multiple times.
    pub fn include_style<P>(&mut self, style_path: P, tailwind: bool)
    where
        P: Into<PathBuf>,
    {
        let style = Style {
            path: style_path.into(),
            tailwind,
            assets_dir: self.assets_dir.clone(),
            tailwind_path: self.tailwind_path.clone(),
        };

        self.styles.insert(style.clone());
        self.included_styles.push(style);
    }
}

#[allow(private_bounds)] // Users never interact with the internal trait, so it's fine
pub trait Asset: DynEq + InternalAsset + Sync + Send {
    fn url(&self) -> Option<String>;
    fn path(&self) -> &PathBuf;

    fn process(&self, _dist_assets_dir: &Path, _tmp_dir: &Path) -> Option<String> {
        None
    }
    fn hash(&self) -> [u8; 8];
}

trait InternalAsset {
    fn assets_dir(&self) -> PathBuf;
}

impl Hash for dyn Asset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash().hash(state);
    }
}

dyn_eq::eq_trait_object!(Asset);

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Image {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
}

impl InternalAsset for Image {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Image {
    fn url(&self) -> Option<String> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        format!("/{}/{}", self.assets_dir().to_string_lossy(), file_name).into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn process(&self, dist_assets_dir: &Path, _: &Path) -> Option<String> {
        fs::copy(
            &self.path,
            dist_assets_dir.join(self.path.file_name().unwrap()),
        )
        .unwrap();

        None
    }

    fn hash(&self) -> [u8; 8] {
        [0; 8]
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Script {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
}

impl InternalAsset for Script {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Script {
    fn url(&self) -> Option<String> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        format!("/{}/{}", self.assets_dir().to_string_lossy(), file_name).into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn hash(&self) -> [u8; 8] {
        // TODO: Proper hash
        [0; 8]
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Style {
    pub path: PathBuf,
    pub(crate) tailwind: bool,
    pub(crate) assets_dir: PathBuf,
    pub(crate) tailwind_path: PathBuf,
}

impl InternalAsset for Style {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Style {
    fn url(&self) -> Option<String> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        format!("/{}/{}", self.assets_dir().to_string_lossy(), file_name).into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn process(&self, _: &Path, tmp_dir: &Path) -> Option<String> {
        // TODO: Detect tailwind automatically
        if self.tailwind {
            let tmp_path = tmp_dir.join(self.path.file_name().unwrap());
            let tmp_path_str = tmp_path.to_str().unwrap().to_string();

            let start_tailwind = SystemTime::now();
            let tailwind_output = Command::new(self.tailwind_path.clone())
                .args(["--input", self.path.to_str().unwrap()])
                .args(["--output", &tmp_path_str])
                .arg("--minify") // TODO: Allow disabling minification
                .output()
                .expect("failed to execute process");

            info!("Tailwind took {:?}", start_tailwind.elapsed().unwrap());

            if tailwind_output.status.success() {
                return Some(tmp_path_str);
            }
        }

        None
    }

    fn hash(&self) -> [u8; 8] {
        // TODO: Proper hash
        [0; 8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_style() {
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            tailwind_path: PathBuf::from("tailwind"),
            ..Default::default()
        };

        page_assets.add_style("style.css", false);

        assert!(page_assets.styles.len() == 1);
    }

    #[test]
    fn test_include_style() {
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            tailwind_path: PathBuf::from("tailwind"),
            ..Default::default()
        };

        page_assets.include_style("style.css", false);

        assert!(page_assets.styles.len() == 1);
        assert!(page_assets.included_styles.len() == 1);
    }

    #[test]
    fn test_add_script() {
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            tailwind_path: PathBuf::from("tailwind"),
            ..Default::default()
        };

        page_assets.add_script("script.js");
        assert!(page_assets.scripts.len() == 1);
    }

    #[test]
    fn test_include_script() {
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            tailwind_path: PathBuf::from("tailwind"),
            ..Default::default()
        };

        page_assets.include_script("script.js");

        assert!(page_assets.scripts.len() == 1);
        assert!(page_assets.included_scripts.len() == 1);
    }

    #[test]
    fn test_add_image() {
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            tailwind_path: PathBuf::from("tailwind"),
            ..Default::default()
        };

        page_assets.add_image("image.png");
        assert!(page_assets.assets.len() == 1);
    }

    #[test]
    fn test_asset_has_leading_slash() {
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            tailwind_path: PathBuf::from("tailwind"),
            ..Default::default()
        };

        let image = page_assets.add_image("image.png");
        assert_eq!(image.url().unwrap().chars().next(), Some('/'));

        let script = page_assets.add_script("script.js");
        assert_eq!(script.url().unwrap().chars().next(), Some('/'));

        let style = page_assets.add_style("style.css", false);
        assert_eq!(style.url().unwrap().chars().next(), Some('/'));
    }
}
