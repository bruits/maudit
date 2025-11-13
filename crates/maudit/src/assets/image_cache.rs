use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use base64::Engine;
use log::debug;
use rustc_hash::FxHashMap;

pub const DEFAULT_IMAGE_CACHE_DIR: &str = "target/maudit_cache/images";
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

#[derive(Debug)]
struct ImageCacheInner {
    manifest: CacheManifest,
    cache_dir: PathBuf,
    manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ImageCache(Arc<Mutex<ImageCacheInner>>);

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCacheInner {
    pub fn new() -> Self {
        Self::with_cache_dir(DEFAULT_IMAGE_CACHE_DIR)
    }

    pub fn with_cache_dir<P: AsRef<Path>>(cache_dir_path: P) -> Self {
        let cache_dir = cache_dir_path.as_ref().to_path_buf();
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

    pub fn save_manifest(&self) {
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
    pub fn get_placeholder(&self, src_path: &Path) -> Option<PlaceholderCacheEntry> {
        let entry = self.manifest.placeholders.get(src_path)?;

        debug!("Placeholder cache hit for {}", src_path.display());
        Some(entry.clone())
    }

    /// Cache a placeholder
    pub fn cache_placeholder(&mut self, src_path: &Path, thumbhash: Vec<u8>) {
        let entry = PlaceholderCacheEntry { thumbhash };

        self.manifest
            .placeholders
            .insert(src_path.to_path_buf(), entry);
        self.save_manifest();
        debug!("Cached placeholder for {}", src_path.display());
    }

    /// Get cached transformed image path or None if not found
    pub fn get_transformed_image(&self, final_filename: &Path) -> Option<PathBuf> {
        let entry = self.manifest.transformed.get(final_filename)?;

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
    pub fn cache_transformed_image(&mut self, final_filename: &Path, cached_path: PathBuf) {
        let entry = TransformedImageCacheEntry {
            cached_path: cached_path.clone(),
        };

        self.manifest
            .transformed
            .insert(final_filename.to_path_buf(), entry);
        self.save_manifest();
        debug!(
            "Cached transformed image {} -> {}",
            final_filename.display(),
            cached_path.display()
        );
    }

    /// Get the cache directory path
    pub fn get_cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Generate a cache path for a transformed image
    pub fn generate_cache_path(&self, final_filename: &Path) -> PathBuf {
        self.cache_dir.join(final_filename)
    }
}

impl ImageCache {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::new())))
    }

    pub fn with_cache_dir<P: AsRef<Path>>(cache_dir_path: P) -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::with_cache_dir(
            cache_dir_path,
        ))))
    }

    fn lock_inner(&'_ self) -> MutexGuard<'_, ImageCacheInner> {
        match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                debug!("ImageCache mutex was poisoned, recovering");
                // This should be fine for our use case because the data won't be corrupted
                poisoned.into_inner()
            }
        }
    }

    /// Get cached placeholder or None if not found
    pub fn get_placeholder(&self, src_path: &Path) -> Option<PlaceholderCacheEntry> {
        self.lock_inner().get_placeholder(src_path)
    }

    /// Cache a placeholder
    pub fn cache_placeholder(&self, src_path: &Path, thumbhash: Vec<u8>) {
        self.lock_inner().cache_placeholder(src_path, thumbhash)
    }

    /// Get cached transformed image path or None if not found
    pub fn get_transformed_image(&self, final_filename: &Path) -> Option<PathBuf> {
        self.lock_inner().get_transformed_image(final_filename)
    }

    /// Cache a transformed image
    pub fn cache_transformed_image(&self, final_filename: &Path, cached_path: PathBuf) {
        self.lock_inner()
            .cache_transformed_image(final_filename, cached_path)
    }

    /// Get the cache directory path
    pub fn get_cache_dir(&self) -> PathBuf {
        self.lock_inner().get_cache_dir().clone()
    }

    /// Generate a cache path for a transformed image
    pub fn generate_cache_path(&self, final_filename: &Path) -> PathBuf {
        self.lock_inner().generate_cache_path(final_filename)
    }

    /// Save the manifest to disk
    pub fn save_manifest(&self) {
        self.lock_inner().save_manifest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_configurable_cache_dir() {
        let custom_cache_dir = env::temp_dir().join("test_maudit_cache");

        // Create cache with custom directory
        let cache = ImageCache::with_cache_dir(&custom_cache_dir);

        // Verify the cache directory is set correctly
        assert_eq!(cache.get_cache_dir(), custom_cache_dir);

        // Test generate_cache_path uses the custom directory
        let test_filename = Path::new("test_image.jpg");
        let cache_path = cache.generate_cache_path(test_filename);
        assert_eq!(cache_path, custom_cache_dir.join(test_filename));
    }

    #[test]
    fn test_default_cache_dir() {
        // Test that the default cache directory is used when no custom dir is set
        let expected_default = PathBuf::from(DEFAULT_IMAGE_CACHE_DIR);

        // Create a new cache instance (will use default)
        let cache = ImageCache::new();
        assert_eq!(cache.get_cache_dir(), expected_default);
    }

    #[test]
    fn test_build_options_integration() {
        use crate::build::options::{AssetsOptions, BuildOptions};

        // Test that BuildOptions can configure the cache directory
        let custom_cache = PathBuf::from("/tmp/custom_maudit_cache");
        let build_options = BuildOptions {
            assets: AssetsOptions {
                image_cache_dir: custom_cache.clone(),
                ..Default::default()
            },
            ..Default::default()
        };

        // Create cache with build options
        let cache = ImageCache::with_cache_dir(&build_options.assets.image_cache_dir);

        // Verify it uses the configured directory
        assert_eq!(cache.get_cache_dir(), custom_cache);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let cache = ImageCache::new();
        let cache_clone = cache.clone();

        // Test that the cache can be shared across threads
        let handle = thread::spawn(move || {
            cache_clone.cache_placeholder(Path::new("test.jpg"), vec![1, 2, 3, 4]);
        });

        handle.join().unwrap();

        // Verify the placeholder was cached
        let entry = cache.get_placeholder(Path::new("test.jpg"));
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().thumbhash, vec![1, 2, 3, 4]);
    }
}
