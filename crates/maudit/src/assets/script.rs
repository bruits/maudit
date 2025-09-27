use std::path::PathBuf;

use crate::assets::{RouteAssetsOptions, make_filename, make_final_path, make_final_url};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Script {
    pub path: PathBuf,
    pub(crate) hash: String,
    pub(crate) included: bool,

    pub(crate) filename: PathBuf,
    pub(crate) url: String,
    pub(crate) build_path: PathBuf,
}

impl Script {
    pub fn new(
        path: PathBuf,
        included: bool,
        hash: String,
        route_assets_options: &RouteAssetsOptions,
    ) -> Self {
        let filename = make_filename(&path, &hash, Some("js"));
        let build_path = make_final_path(&route_assets_options.output_assets_dir, &filename);
        let url = make_final_url(&route_assets_options.assets_dir, &filename);

        Self {
            path,
            hash,
            included,
            filename,
            url,
            build_path,
        }
    }
}
