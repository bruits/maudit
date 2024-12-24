use dyn_eq::DynEq;
use std::hash::Hash;
use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Default)]
pub struct PageAssets(pub(crate) HashSet<Box<dyn Asset>>);

impl PageAssets {
    pub fn add_image(&mut self, image_path: PathBuf) -> Image {
        let image = Box::new(Image { path: image_path });

        self.0.insert(image.clone());

        *image
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
