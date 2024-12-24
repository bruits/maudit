use dyn_eq::DynEq;
use rustc_hash::FxHashSet;
use std::hash::Hash;
use std::{fs, path::PathBuf};

#[derive(Default)]
pub struct PageAssets {
    pub(crate) assets: FxHashSet<Box<dyn Asset>>,
    pub(crate) scripts: FxHashSet<Script>,
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
}

pub trait Asset: DynEq {
    fn url(&self) -> Option<String>;
    fn path(&self) -> &PathBuf;

    fn process(&self);
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

    fn process(&self) {
        fs::copy(
            &self.path,
            "dist/_assets/".to_string() + self.path.file_name().unwrap().to_str().unwrap(),
        )
        .unwrap();
    }

    fn hash(&self) -> [u8; 8] {
        [0; 8]
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

    fn process(&self) {}

    fn hash(&self) -> [u8; 8] {
        [0; 8]
    }
}
