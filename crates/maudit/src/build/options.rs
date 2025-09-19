use std::path::PathBuf;

use crate::{assets::PageAssetsOptions, is_dev};

/// Maudit build options. Should be passed to [`coronate()`](crate::coronate()).
///
/// ## Examples
/// Default values:
/// ```rs
/// use maudit::{
///  content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![],
///     BuildOptions::default(),
///   )
/// }
/// ```
/// Custom values:
/// ```rs
/// use maudit::{
///   content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![],
///     BuildOptions {
///       output_dir: "public".to_string(),
///       static_dir: "static".to_string(),
///       assets: AssetsOptions {
///         assets_dir: "_assets".to_string(),
///         tailwind_binary_path: "./node_modules/.bin/tailwindcss".to_string(),
///         ..Default::default()
///       },
///       ..Default::default()
///     },
///   )
/// }
/// ```
pub struct BuildOptions {
    pub output_dir: PathBuf,
    pub static_dir: PathBuf,

    /// Whether to clean the output directory before building.
    ///
    /// At the speed Maudit operates at, not cleaning the output directory may offer a significant performance improvement at the cost of potentially serving stale content.
    pub clean_output_dir: bool,

    pub assets: AssetsOptions,
}

impl BuildOptions {
    /// Returns the fully resolved assets options, with the `assets_dir` set to be inside the `output_dir`.
    /// e.g. if `output_dir` is `dist` and `assets.assets_dir` is `_maudit`, this will return `dist/_maudit`.
    pub fn page_assets_options(&self) -> PageAssetsOptions {
        PageAssetsOptions {
            assets_dir: self.output_dir.join(&self.assets.assets_dir),
            hashing_strategy: self.assets.hashing_strategy,
        }
    }
}

#[derive(Clone)]
pub struct AssetsOptions {
    /// Path to [the TailwindCSS CLI binary](https://tailwindcss.com/docs/installation/tailwind-cli). By default `tailwindcss`, which assumes you've installed it globally (for example, through Homebrew) and that it is in your `PATH`.
    ///
    /// This is commonly set to `./node_modules/.bin/tailwindcss` or similar, in order to use a locally installed version.
    pub tailwind_binary_path: PathBuf,

    /// Directory inside the output directory to place built assets in. This directory will be created if it doesn't exist.
    /// Defaults to `_maudit`.
    ///
    /// Note that this value is not automatically joined with the `output_dir` in `BuildOptions`. Use [`BuildOptions::page_assets_options()`] to get a `PageAssetsOptions` with the correct final path.
    pub assets_dir: PathBuf,

    /// Strategy to use when hashing assets for fingerprinting.
    ///
    /// Defaults to [`AssetHashingStrategy::Precise`] in production builds, and [`AssetHashingStrategy::FastImprecise`] in development builds.
    pub hashing_strategy: AssetHashingStrategy,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum AssetHashingStrategy {
    /// Hash assets based on their full content, path and options (e.g. whether TailwindCSS is enabled for styles).
    Precise,
    /// Hash assets based on their modified time, size, path and options. This is much faster, but may lead to stale assets and sometimes unnecessary rebuilds.
    FastImprecise,
}

impl Default for AssetsOptions {
    fn default() -> Self {
        Self {
            tailwind_binary_path: "tailwindcss".into(),
            assets_dir: "_maudit".into(),
            hashing_strategy: if is_dev() {
                AssetHashingStrategy::FastImprecise
            } else {
                AssetHashingStrategy::Precise
            },
        }
    }
}

/// Provides default values for [`crate::coronate()`]. Designed to work for most projects.
///
/// ## Examples
/// ```rs
/// use maudit::{
///  content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![],
///     BuildOptions::default(),
///   )
/// }
/// ```
impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            output_dir: "dist".into(),
            static_dir: "static".into(),
            clean_output_dir: true,
            assets: AssetsOptions::default(),
        }
    }
}
