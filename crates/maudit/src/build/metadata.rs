use std::{
    path::{Path, PathBuf},
    process::Termination,
    time::Instant,
};

use rustc_hash::FxHashMap;

/// Metadata returned by [`coronate()`](crate::coronate) for a single page after a successful build.
#[derive(Debug)]
pub struct PageOutput {
    pub route: String,
    pub file_path: String,
    pub params: Option<FxHashMap<String, Option<String>>>,
    /// Whether this page was served from the incremental build cache
    /// (i.e., not re-rendered because its dependencies didn't change).
    pub cached: bool,
}

/// Metadata returned by [`coronate()`](crate::coronate) for a single static asset after a successful build.
///
/// A static asset is a file that is copied to the output directory without any processing.
#[derive(Debug)]
pub struct StaticAssetOutput {
    pub file_path: String,
    pub original_path: String,
}

/// Metadata returned by [`coronate()`](crate::coronate) after a successful build.
#[derive(Debug)]
pub struct BuildOutput {
    pub start_time: Instant,
    pub pages: Vec<PageOutput>,
    pub assets: Vec<String>,
    pub static_files: Vec<StaticAssetOutput>,
    pub(crate) removed_pages: usize,
    /// Map from rendered asset URL to the resolved URL coronate wrote to disk.
    /// Used by [`AssetUrl::resolve`](crate::assets::AssetUrl::resolve) to look up
    /// the post-build URL.
    pub(crate) url_substitutions: FxHashMap<String, String>,
    /// Map from rendered asset on-disk path to the resolved path. Used by
    /// [`AssetPath::resolve`](crate::assets::AssetPath::resolve).
    pub(crate) path_substitutions: FxHashMap<PathBuf, PathBuf>,
}

impl BuildOutput {
    pub fn new(start_time: Instant) -> Self {
        Self {
            start_time,
            pages: Vec::new(),
            assets: Vec::new(),
            static_files: Vec::new(),
            removed_pages: 0,
            url_substitutions: FxHashMap::default(),
            path_substitutions: FxHashMap::default(),
        }
    }

    pub(crate) fn add_page(
        &mut self,
        route: String,
        file_path: String,
        params: Option<FxHashMap<String, Option<String>>>,
        cached: bool,
    ) {
        self.pages.push(PageOutput {
            route,
            file_path,
            params,
            cached,
        });
    }

    pub(crate) fn add_asset(&mut self, file_path: String) {
        self.assets.push(file_path);
    }

    pub(crate) fn add_static_file(&mut self, file_path: String, original_path: String) {
        self.static_files.push(StaticAssetOutput {
            file_path,
            original_path,
        });
    }

    pub(crate) fn record_asset_substitution(
        &mut self,
        url_from: String,
        url_to: String,
        path_from: PathBuf,
        path_to: PathBuf,
    ) {
        self.url_substitutions.insert(url_from, url_to);
        self.path_substitutions.insert(path_from, path_to);
    }

    /// Resolve a rendered asset URL to its post-build form. Returns `None` when
    /// no substitution was recorded (the URL is already final).
    pub fn resolve_asset_url(&self, rendered: &str) -> Option<&str> {
        self.url_substitutions.get(rendered).map(String::as_str)
    }

    /// Resolve a rendered asset path to its post-build form. Returns `None` when
    /// no substitution was recorded.
    pub fn resolve_asset_path(&self, rendered: &Path) -> Option<&Path> {
        self.path_substitutions.get(rendered).map(PathBuf::as_path)
    }

    /// Returns true if any page was added, changed, or removed during this build.
    ///
    /// Useful for deciding whether post-build work needs to run or can be skipped.
    pub fn has_changes(&self) -> bool {
        self.removed_pages > 0 || self.pages.iter().any(|p| !p.cached)
    }
}

impl Default for BuildOutput {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

impl Termination for BuildOutput {
    fn report(self) -> std::process::ExitCode {
        0.into()
    }
}
