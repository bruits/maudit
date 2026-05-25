use std::path::PathBuf;

use crate::assets::{
    IntermediateUrlFormat, RouteAssetsOptions, make_filename, make_final_path, make_final_url,
    make_pending_path, make_pending_url,
};

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct StyleOptions {
    pub tailwind: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
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
        hash: String,
        route_assets_options: &RouteAssetsOptions,
    ) -> Self {
        let filename = make_filename(&path, &hash, Some("css"));
        let (url, build_path) = match route_assets_options.intermediate_url_format {
            IntermediateUrlFormat::SourceHash => (
                make_final_url(&route_assets_options.assets_dir, &filename),
                make_final_path(&route_assets_options.output_assets_dir, &filename),
            ),
            IntermediateUrlFormat::Placeholder => (
                make_pending_url(&filename),
                make_pending_path(&route_assets_options.output_dir, &filename),
            ),
        };

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
