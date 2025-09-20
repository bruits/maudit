use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use base64::Engine;
use log::debug;
use rustc_hash::FxHashMap;

// TODO: Make this configurable
pub const IMAGE_CACHE_DIR: &str = "target/maudit_cache/images";
pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct PlaceholderCacheEntry {
    pub thumbhash: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TransformedImageCacheEntry {
    /// Path to the cached transformed image file
    pub cached_path: PathBuf,
}

#[derive(Debug, Default)]
struct CacheManifest {
    /// Cache for placeholder data (thumbhash, etc.)
    placeholders: FxHashMap<PathBuf, PlaceholderCacheEntry>,
    /// Cache for transformed images (path + options -> cached file path)
    transformed: FxHashMap<PathBuf, TransformedImageCacheEntry>,
}

pub struct ImageCache {
    manifest: CacheManifest,
    cache_dir: PathBuf,
    manifest_path: PathBuf,
}

static CACHE: OnceLock<Mutex<ImageCache>> = OnceLock::new();

impl ImageCache {
    fn new() -> Self {
        let cache_dir = PathBuf::from(IMAGE_CACHE_DIR);
        let manifest_path = cache_dir.join("manifest");

        // Create cache directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            debug!("Failed to create cache directory: {}", e);
        }

        // Load existing manifest or create new one
        let manifest = if manifest_path.exists() {
            Self::load_manifest(&manifest_path).unwrap_or_default()
        } else {
            CacheManifest::default()
        };

        debug!(
            "Image cache initialized with {} placeholders and {} transformed images",
            manifest.placeholders.len(),
            manifest.transformed.len()
        );

        Self {
            manifest,
            cache_dir,
            manifest_path,
        }
    }

    fn load_manifest(path: &Path) -> Option<CacheManifest> {
        let content = fs::read_to_string(path).ok()?;
        let mut manifest = CacheManifest::default();
        let mut found_version = None;

        let mut current_section = "";
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for version line
            if line.starts_with("version = ") {
                if let Some(version_str) = line.strip_prefix("version = ")
                    && let Ok(version) = version_str.parse::<u32>()
                {
                    found_version = Some(version);
                }
                continue;
            }

            if line == "[placeholders]" {
                current_section = "placeholders";
                continue;
            } else if line == "[transformed]" {
                current_section = "transformed";
                continue;
            }

            match current_section {
                "placeholders" => {
                    if let Some((path_str, thumbhash_b64)) = line.split_once('=')
                        && let Ok(thumbhash) =
                            base64::engine::general_purpose::STANDARD.decode(thumbhash_b64)
                    {
                        let entry = PlaceholderCacheEntry { thumbhash };
                        manifest.placeholders.insert(PathBuf::from(path_str), entry);
                    }
                }
                "transformed" => {
                    if let Some((cache_key, cached_path_str)) = line.split_once('=') {
                        let entry = TransformedImageCacheEntry {
                            cached_path: PathBuf::from(cached_path_str),
                        };
                        manifest.transformed.insert(PathBuf::from(cache_key), entry);
                    }
                }
                _ => {}
            }
        }

        // Check version compatibility
        if let Some(version) = found_version {
            if version != MANIFEST_VERSION {
                debug!(
                    "Manifest version mismatch: found {}, expected {}. Invalidating cache.",
                    version, MANIFEST_VERSION
                );
                // Delete the manifest file to invalidate the cache
                let _ = fs::remove_file(path);
                return None;
            }
        } else {
            debug!("No version found in manifest. Invalidating cache.");
            let _ = fs::remove_file(path);
            return None;
        }

        Some(manifest)
    }

    fn get() -> &'static Mutex<ImageCache> {
        CACHE.get_or_init(|| Mutex::new(ImageCache::new()))
    }

    fn save_manifest(&self) {
        let mut content = String::new();
        content.push_str("# Maudit Image Cache Manifest\n");
        content.push_str(&format!("version = {}\n\n", MANIFEST_VERSION));

        // Write placeholders section
        content.push_str("[placeholders]\n");
        for (path, entry) in &self.manifest.placeholders {
            let thumbhash_b64 = base64::engine::general_purpose::STANDARD.encode(&entry.thumbhash);
            content.push_str(&format!("{}={}\n", path.to_string_lossy(), thumbhash_b64));
        }

        content.push_str("\n[transformed]\n");
        for (cache_key, entry) in &self.manifest.transformed {
            content.push_str(&format!(
                "{}={}\n",
                cache_key.to_string_lossy(),
                entry.cached_path.to_string_lossy()
            ));
        }

        if let Err(e) = fs::write(&self.manifest_path, content) {
            debug!("Failed to save cache manifest: {}", e);
        }
    }

    /// Get cached placeholder or None if not found
    pub fn get_placeholder(src_path: &Path) -> Option<PlaceholderCacheEntry> {
        let cache = Self::get().lock().ok()?;
        let entry = cache.manifest.placeholders.get(src_path)?;

        debug!("Placeholder cache hit for {}", src_path.display());
        Some(entry.clone())
    }

    /// Cache a placeholder
    pub fn cache_placeholder(src_path: &Path, thumbhash: Vec<u8>) {
        if let Ok(mut cache) = Self::get().lock() {
            let entry = PlaceholderCacheEntry { thumbhash };

            cache
                .manifest
                .placeholders
                .insert(src_path.to_path_buf(), entry);
            cache.save_manifest();
            debug!("Cached placeholder for {}", src_path.display());
        }
    }

    /// Get cached transformed image path or None if not found
    pub fn get_transformed_image(final_filename: &Path) -> Option<PathBuf> {
        let cache = Self::get().lock().ok()?;
        let entry = cache.manifest.transformed.get(final_filename)?;

        // Check if cached file still exists
        if !entry.cached_path.exists() {
            debug!(
                "Cached transformed image file missing: {}",
                entry.cached_path.display()
            );
            return None;
        }

        debug!(
            "Transformed image cache hit for {} -> {}",
            final_filename.display(),
            entry.cached_path.display()
        );
        Some(entry.cached_path.clone())
    }

    /// Cache a transformed image
    pub fn cache_transformed_image(final_filename: &Path, cached_path: PathBuf) {
        if let Ok(mut cache) = Self::get().lock() {
            let entry = TransformedImageCacheEntry {
                cached_path: cached_path.clone(),
            };

            cache
                .manifest
                .transformed
                .insert(final_filename.to_path_buf(), entry);
            cache.save_manifest();
            debug!(
                "Cached transformed image {} -> {}",
                final_filename.display(),
                cached_path.display()
            );
        }
    }

    /// Generate a cache path for a transformed image
    pub fn generate_cache_path(final_filename: &Path) -> PathBuf {
        if let Ok(cache) = Self::get().lock() {
            cache.cache_dir.join(final_filename)
        } else {
            // Fallback path if cache is unavailable
            PathBuf::from(IMAGE_CACHE_DIR).join(final_filename)
        }
    }
}
