use std::path::PathBuf;

use crate::assets::{
    HashAssetType, HashConfig, RouteAssetsOptions, calculate_hash, make_filename, make_final_path,
    make_final_url,
};

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct StyleOptions {
    pub tailwind: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Style {
    pub path: PathBuf,
    pub(crate) hash: String,
    pub(crate) tailwind: bool,
    pub(crate) included: bool,

    pub(crate) filename: PathBuf,
    pub(crate) url: String,
    pub(crate) build_path: PathBuf,
}

impl Style {
    pub fn new(
        path: PathBuf,
        included: bool,
        style_options: &StyleOptions,
        page_assets_options: &RouteAssetsOptions,
    ) -> Self {
        let hash = calculate_hash(
            &path,
            Some(&HashConfig {
                asset_type: HashAssetType::Style(style_options),
                hashing_strategy: &page_assets_options.hashing_strategy,
            }),
        );

        let filename = make_filename(&path, &hash, Some("css"));
        let build_path = make_final_path(&page_assets_options.output_assets_dir, &filename);
        let url = make_final_url(&page_assets_options.assets_dir, &filename);

        Self {
            path,
            tailwind: style_options.tailwind,
            hash,
            included,
            filename,
            url,
            build_path,
        }
    }
}
