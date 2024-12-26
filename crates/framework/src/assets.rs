use dyn_eq::DynEq;
use log::info;
use maud::{html, Markup, Render};
use rustc_hash::FxHashSet;
use std::hash::Hash;
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
}

impl PageAssets {
    pub fn add_image(&mut self, image_path: PathBuf) -> Image {
        let image = Box::new(Image { path: image_path });

        self.assets.insert(image.clone());

        *image
    }

    pub fn add_script(&mut self, script_path: PathBuf) -> Script {
        let script = Script { path: script_path };

        self.scripts.insert(script.clone());

        script
    }

    pub fn include_script(&mut self, script_path: PathBuf) {
        let script = Script { path: script_path };

        self.scripts.insert(script.clone());
        self.included_scripts.push(script);
    }

    pub fn add_style(&mut self, style_path: PathBuf, tailwind: bool) -> Style {
        let style = Style {
            path: style_path,
            tailwind,
        };

        self.styles.insert(style.clone());

        style
    }

    pub fn include_style(&mut self, style_path: PathBuf, tailwind: bool) {
        let style = Style {
            path: style_path,
            tailwind,
        };

        self.styles.insert(style.clone());
        self.included_styles.push(style);
    }
}

pub trait Asset: DynEq {
    fn url(&self) -> Option<String>;
    fn path(&self) -> &PathBuf;

    fn process(&self) -> Option<String> {
        None
    }
    fn hash(&self) -> [u8; 8];
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
}

impl Asset for Image {
    fn url(&self) -> Option<String> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        format!("/_assets/{}", file_name).into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn process(&self) -> Option<String> {
        fs::copy(
            &self.path,
            "dist/_assets/".to_string() + self.path.file_name().unwrap().to_str().unwrap(),
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
}

impl Asset for Script {
    fn url(&self) -> Option<String> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        format!("/_assets/{}", file_name).into()
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
}

impl Asset for Style {
    fn url(&self) -> Option<String> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        format!("/_assets/{}", file_name).into()
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn process(&self) -> Option<String> {
        // TODO: Detect tailwind automatically
        if self.tailwind {
            let tmp_path = "dist/_tmp/tailwind.css";
            let start_tailwind = SystemTime::now();
            let tailwind_output = Command::new("tailwindcss") // TODO: Allow custom tailwind binary path
                .arg("--minify") // TODO: Allow disabling minification
                .args(["--output", tmp_path])
                .output()
                .expect("failed to execute process");

            info!("Tailwind took {:?}", start_tailwind.elapsed().unwrap());

            if tailwind_output.status.success() {
                return Some(tmp_path.into());
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
