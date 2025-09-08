use std::{process::Termination, time::Instant};

use rustc_hash::FxHashMap;

/// Metadata returned by [`coronate()`](crate::coronate) for a single page after a successful build.
#[derive(Debug)]
pub struct PageOutput {
    pub route: String,
    pub file_path: String,
    pub params: Option<FxHashMap<String, String>>,
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
}

impl BuildOutput {
    pub fn new(start_time: Instant) -> Self {
        Self {
            start_time,
            pages: Vec::new(),
            assets: Vec::new(),
            static_files: Vec::new(),
        }
    }

    pub(crate) fn add_page(
        &mut self,
        route: String,
        file_path: String,
        params: Option<FxHashMap<String, String>>,
    ) {
        self.pages.push(PageOutput {
            route,
            file_path,
            params,
        });
    }

    // TODO
    #[allow(dead_code)]
    pub(crate) fn add_asset(&mut self, file_path: String) {
        self.assets.push(file_path);
    }

    pub(crate) fn add_static_file(&mut self, file_path: String, original_path: String) {
        self.static_files.push(StaticAssetOutput {
            file_path,
            original_path,
        });
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
