use std::path::PathBuf;

use crate::assets::{Asset, InternalAsset};

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Script {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) output_assets_dir: PathBuf,
    pub(crate) hash: String,
    pub(crate) included: bool,
}

impl InternalAsset for Script {
    fn assets_dir(&self) -> &PathBuf {
        &self.assets_dir
    }

    fn output_assets_dir(&self) -> &PathBuf {
        &self.output_assets_dir
    }
}

impl Asset for Script {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn hash(&self) -> String {
        self.hash.clone()
    }

    fn final_extension(&self) -> String {
        let current_extension = self
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();

        match current_extension {
            "ts" => "js",
            ext => ext,
        }
        .to_string()
    }
}
