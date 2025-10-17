use std::path::PathBuf;

use crate::{assets::RouteAssetsOptions, is_dev};

/// Maudit build options. Should be passed to [`coronate()`](crate::coronate()).
///
/// ## Examples
/// Default values:
/// ```rust
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
/// ```rust
/// use maudit::{
///   content_sources, coronate, routes, BuildOptions, BuildOutput, AssetsOptions,
/// };
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![],
///     BuildOptions {
///       output_dir: "public".into(),
///       static_dir: "static".into(),
///       assets: AssetsOptions {
///         assets_dir: "_assets".into(),
///         tailwind_binary_path: "./node_modules/.bin/tailwindcss".into(),
///         ..Default::default()
///       },
///       ..Default::default()
///     },
///   )
/// }
/// ```
pub struct BuildOptions {
    /// Base URL for the site, e.g. `https://example.com` or `https://example.com/subdir`.
    /// This value is used to generate canonical URLs and can be used wherever the full site URL is needed (e.g. in SEO meta tags) through [`PageContext::base_url`](crate::route::PageContext::base_url) in pages.
    pub base_url: Option<String>,

    pub output_dir: PathBuf,
    pub static_dir: PathBuf,

    /// Whether to clean the output directory before building.
    ///
    /// At the speed Maudit operates at, not cleaning the output directory may offer a significant performance improvement at the cost of potentially serving stale content.
    pub clean_output_dir: bool,

    /// Path to the Maudit sidecar binary. If not set, will use `./node_modules/.bin/maudit-node`.
    pub node_sidecar_path: PathBuf,

    pub assets: AssetsOptions,
}

impl BuildOptions {
    /// Returns the fully resolved assets options, with the `output_assets_dir` property resolved to be inside `output_dir`.
    /// e.g. if `output_dir` is `dist` and `assets.assets_dir` is `_maudit`, `output_assets_dir` will return `dist/_maudit`. The user-entered `assets.assets_dir` is also available and unchanged.
    pub fn route_assets_options(&self) -> RouteAssetsOptions {
        RouteAssetsOptions {
            assets_dir: self.assets.assets_dir.clone(),
            output_assets_dir: self.output_dir.join(&self.assets.assets_dir),
            hashing_strategy: self.assets.hashing_strategy,
        }
    }
}

#[derive(Clone)]
pub struct AssetsOptions {
    /// Directory inside the output directory to place built assets in.
    /// Defaults to `_maudit`.
    ///
    /// Note that this value is not automatically joined with the `output_dir` in `BuildOptions`. Use [`BuildOptions::route_assets_options()`] to get a `RouteAssetsOptions` with the correct final path.
    pub assets_dir: PathBuf,

    /// Strategy to use when hashing assets for fingerprinting.
    ///
    /// Defaults to [`AssetHashingStrategy::Precise`] in production builds, and [`AssetHashingStrategy::FastImprecise`] in development builds. Note that this means that the cache isn't shared between dev and prod builds by default, if you have a lot of assets you may want to set this to the same value in both environments.
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
/// ```rust
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
            base_url: None,
            output_dir: "dist".into(),
            static_dir: "static".into(),
            clean_output_dir: true,
            node_sidecar_path: "./node_modules/.bin/maudit-node".into(),
            assets: AssetsOptions::default(),
        }
    }
}
