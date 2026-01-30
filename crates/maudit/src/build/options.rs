use std::{fs, path::PathBuf};

use crate::{assets::RouteAssetsOptions, is_dev, sitemap::SitemapOptions};

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
///   PrefetchOptions, PrefetchStrategy,
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
///       prefetch: PrefetchOptions {
///         strategy: PrefetchStrategy::Viewport,
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

    /// Whether to enable incremental builds.
    ///
    /// When enabled, Maudit tracks which assets are used by which routes and only rebuilds
    /// routes affected by changed files. This can significantly speed up rebuilds when only
    /// a few files have changed.
    ///
    /// Defaults to `true` in dev mode (`maudit dev`) and `false` in production builds.
    pub incremental: bool,

    /// Directory for build cache storage (incremental build state, etc.).
    ///
    /// Defaults to `target/maudit_cache/{package_name}` where `{package_name}` is derived
    /// from the current directory name.
    pub cache_dir: PathBuf,

    /// Directory for caching processed assets (images, etc.).
    ///
    /// If `None`, defaults to `{cache_dir}/assets`.
    pub assets_cache_dir: Option<PathBuf>,

    pub assets: AssetsOptions,

    pub prefetch: PrefetchOptions,

    /// Options for sitemap generation. See [`SitemapOptions`] for configuration.
    pub sitemap: SitemapOptions,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PrefetchStrategy {
    /// No prefetching
    None,
    /// Prefetch links when users hover over them (with 80ms delay)
    Hover,
    /// Prefetch links when users click/tap on them
    Tap,
    /// Prefetch all links currently visible in the viewport
    Viewport,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PrerenderEagerness {
    /// Prerender as soon as possible
    Immediate,
    /// Prerender eagerly but not immediately
    Eager,
    /// Prerender with moderate eagerness
    Moderate,
    /// Prerender conservatively
    Conservative,
}

#[derive(Clone)]
pub struct PrefetchOptions {
    /// The prefetch strategy to use
    pub strategy: PrefetchStrategy,
    /// Enable prerendering using Speculation Rules API if supported.
    pub prerender: bool,
    /// Hint to the browser as to how eagerly it should prefetch/prerender.
    /// Only works when prerender is enabled and browser supports Speculation Rules API.
    pub eagerness: PrerenderEagerness,
}

impl Default for PrefetchOptions {
    fn default() -> Self {
        Self {
            strategy: PrefetchStrategy::Tap,
            prerender: false,
            eagerness: PrerenderEagerness::Immediate,
        }
    }
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

    /// Returns the directory for caching processed assets (images, etc.).
    /// Uses `assets_cache_dir` if set, otherwise defaults to `{cache_dir}/assets`.
    pub fn assets_cache_dir(&self) -> PathBuf {
        self.assets_cache_dir
            .clone()
            .unwrap_or_else(|| self.cache_dir.join("assets"))
    }
}

#[derive(Clone)]
pub struct AssetsOptions {
    /// Path to [the TailwindCSS CLI binary](https://tailwindcss.com/docs/installation/tailwind-cli). By default `tailwindcss`, which assumes you've installed it globally (for example, through Homebrew) and that it is in your `PATH`.
    ///
    /// This is commonly set to `./node_modules/.bin/tailwindcss` or similar, in order to use a locally installed version.
    pub tailwind_binary_path: PathBuf,

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
        let site_name = get_site_name();
        let cache_dir = find_target_dir()
            .unwrap_or_else(|_| PathBuf::from("target"))
            .join("maudit_cache")
            .join(&site_name);

        Self {
            base_url: None,
            output_dir: "dist".into(),
            static_dir: "static".into(),
            clean_output_dir: true,
            incremental: is_dev(),
            cache_dir,
            assets_cache_dir: None,
            prefetch: PrefetchOptions::default(),
            assets: AssetsOptions::default(),
            sitemap: SitemapOptions::default(),
        }
    }
}

/// Get the site name for cache directory purposes.
///
/// Uses the current executable's name (which matches the package/binary name),
/// falling back to the current directory name.
fn get_site_name() -> String {
    // Get the binary name from the current executable
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| {
            // Fallback to current directory name
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "default".to_string())
        })
}

/// Find the target directory using multiple strategies
///
/// This function tries multiple approaches to locate the target directory:
/// 1. CARGO_TARGET_DIR / CARGO_BUILD_TARGET_DIR environment variables
/// 2. Local ./target directory
/// 3. Workspace root target directory (walking up to find [workspace])
/// 4. Fallback to relative "target" path
fn find_target_dir() -> Result<PathBuf, std::io::Error> {
    // 1. Check CARGO_TARGET_DIR and CARGO_BUILD_TARGET_DIR environment variables
    for env_var in ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"] {
        if let Ok(target_dir) = std::env::var(env_var) {
            let path = PathBuf::from(&target_dir);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // 2. Look for target directory in current directory
    let local_target = PathBuf::from("target");
    if local_target.exists() {
        return Ok(local_target);
    }

    // 3. Try to find workspace root by looking for Cargo.toml with [workspace]
    let mut current = std::env::current_dir()?;
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]")
        {
            let workspace_target = current.join("target");
            if workspace_target.exists() {
                return Ok(workspace_target);
            }
        }

        // Move up to parent directory
        if !current.pop() {
            break;
        }
    }

    // 4. Final fallback to relative path
    Ok(PathBuf::from("target"))
}
