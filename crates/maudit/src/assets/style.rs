use std::path::{Path, PathBuf};

use crate::assets::{RouteAssetsOptions, make_filename, make_final_path};

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
        let build_path = make_final_path(&route_assets_options.output_assets_dir, &filename);
        // Placeholder URL; replaced after CSS bundling with a content-hashed final URL
        // so that Tailwind-scanned classes (which change the bundled bytes without
        // changing the source file) cascade into a new filename.
        let url = make_placeholder_url(&route_assets_options.assets_dir, &hash);

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

pub(crate) fn make_placeholder_url(assets_dir: &Path, source_hash: &str) -> String {
    format!(
        "/{}/__maudit_style_{}__.css",
        assets_dir.display(),
        source_hash
    )
}
