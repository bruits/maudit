use std::path::PathBuf;

use crate::assets::{
    HashAssetType, HashConfig, RouteAssetsOptions, calculate_hash, make_filename, make_final_path,
    make_final_url,
};

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Script {
    pub path: PathBuf,
    pub(crate) hash: String,
    pub(crate) included: bool,

    pub(crate) filename: PathBuf,
    pub(crate) url: String,
    pub(crate) build_path: PathBuf,
}

impl Script {
    pub fn new(path: PathBuf, included: bool, route_assets_options: &RouteAssetsOptions) -> Self {
        let hash = calculate_hash(
            &path,
            Some(&HashConfig {
                asset_type: HashAssetType::Script,
                hashing_strategy: &route_assets_options.hashing_strategy,
            }),
        );

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
