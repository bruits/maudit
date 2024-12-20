use std::fmt::Display;
use std::fs::{self};
use std::path::PathBuf;

pub struct Asset {
    file_path: PathBuf,
    final_url: String,
}

trait GenericAsset {
    fn load(file_path: PathBuf) -> Self;
}

impl GenericAsset for Asset {
    fn load(file_path: PathBuf) -> Self {
        let canonicalized = file_path.canonicalize().unwrap();
        let file_name = canonicalized.file_name().unwrap().to_string_lossy();

        Asset {
            file_path: canonicalized.clone(),
            final_url: format!("/_assets/{}", file_name),
        }
    }
}

impl Asset {
    pub fn new(file_path: PathBuf) -> Self {
        let asset = Asset::load(file_path);
        asset.finalize();

        asset
    }

    fn finalize(&self) {
        fs::copy(
            &self.file_path,
            format!(
                "dist/_assets/{}",
                self.file_path.file_name().unwrap().to_string_lossy()
            ),
        )
        .unwrap();
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.final_url)
    }
}
