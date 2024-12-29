use dyn_eq::DynEq;
use log::info;
use maud::{html, Markup, Render};
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
}

impl PageAssets {
    pub fn add_image<P>(&mut self, image_path: P) -> Image
    where
        P: Into<PathBuf>,
    {
        let image = Box::new(Image {
            path: image_path.into(),
            assets_dir: self.assets_dir.clone(),
        });

        self.assets.insert(image.clone());

        *image
    }

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

    pub fn add_style<P>(&mut self, style_path: P, tailwind: bool) -> Style
    where
        P: Into<PathBuf>,
    {
        let style = Style {
            path: style_path.into(),
            tailwind,
            assets_dir: self.assets_dir.clone(),
        };

        self.styles.insert(style.clone());

        style
    }

    pub fn include_style<P>(&mut self, style_path: P, tailwind: bool)
    where
        P: Into<PathBuf>,
    {
        let style = Style {
            path: style_path.into(),
            tailwind,
            assets_dir: self.assets_dir.clone(),
        };

        self.styles.insert(style.clone());
        self.included_styles.push(style);
    }
}

#[allow(private_bounds)] // Users never interact with the internal trait, so it's fine
pub trait Asset: DynEq + InternalAsset {
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

impl Render for Image {
    fn render(&self) -> Markup {
        html! {
            img src=(self.url().unwrap()) loading="lazy" decoding="async";
        }
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

impl Render for Script {
    fn render(&self) -> Markup {
        html! {
            script src=(self.url().unwrap()) r#type="module" {}
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Style {
    pub path: PathBuf,
    pub(crate) tailwind: bool,
    pub(crate) assets_dir: PathBuf,
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
            let file_name = self.path.file_name().unwrap().to_str().unwrap();
            let tmp_path = tmp_dir.join(file_name);
            let tmp_path_str = tmp_path.to_str().unwrap().to_string();

            let start_tailwind = SystemTime::now();
            let tailwind_output = Command::new("tailwindcss") // TODO: Allow custom tailwind binary path
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

impl Render for Style {
    fn render(&self) -> Markup {
        html! {
            link rel="stylesheet" type="text/css" href=(self.url().unwrap());
        }
    }
}
