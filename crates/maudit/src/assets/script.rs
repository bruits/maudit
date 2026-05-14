use std::path::{Path, PathBuf};

use crate::assets::{RouteAssetsOptions, make_filename, make_final_path};

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
        // URL is a deterministic placeholder; the real filename is determined by Rolldown's
        // content hash after bundling (so it reflects transitive asset deps like WASM imports).
        // A post-bundle pass rewrites these placeholders in any HTML that referenced them.
        let url = make_placeholder_url(&route_assets_options.assets_dir, &hash);

        Self {
            path,
            hash,
            included,
            filename,
            url,
            build_path,
        }
    }

    /// The chunk name passed to Rolldown's `[name]` filename pattern. Encodes the source-content
    /// hash so we can match `result.assets` entries back to their originating Script.
    pub(crate) fn chunk_name(&self) -> String {
        self.filename
            .with_extension("")
            .to_string_lossy()
            .into_owned()
    }
}

/// Token embedded in HTML during render in place of a script's final URL. Replaced after
/// bundling with `/<assets_dir>/<rolldown-emitted-filename>`. URL-shaped so it survives any
/// templating layer that expects a path.
pub(crate) fn make_placeholder_url(assets_dir: &Path, source_hash: &str) -> String {
    format!(
        "/{}/__maudit_script_{}__.js",
        assets_dir.display(),
        source_hash
    )
}
