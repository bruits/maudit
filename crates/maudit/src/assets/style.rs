use std::path::PathBuf;

use crate::assets::{Asset, InternalAsset};

#[derive(Clone, PartialEq, Eq, Hash, Default)]
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
    pub(crate) included: bool,
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
}
