use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use log::debug;
use rustc_hash::FxHashMap;

use crate::build::cache::BuildCache;

#[derive(Debug, Clone)]
pub struct PlaceholderCacheEntry {
    pub thumbhash: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TransformedImageCacheEntry {
    /// Path to the cached transformed image file
    pub cached_path: PathBuf,
}

#[derive(Debug)]
struct ImageCacheInner {
    /// Cache for placeholder data (thumbhash, etc.)
    placeholders: FxHashMap<PathBuf, PlaceholderCacheEntry>,
    /// Cache for transformed images (final_filename -> cached file path)
    transformed: FxHashMap<PathBuf, TransformedImageCacheEntry>,
    /// Directory where actual processed image files are stored
    cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ImageCache(Arc<Mutex<ImageCacheInner>>);

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCacheInner {
    pub fn new(cache_dir: PathBuf) -> Self {
        // Create cache directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            debug!("Failed to create cache directory: {}", e);
        }

        Self {
            placeholders: FxHashMap::default(),
            transformed: FxHashMap::default(),
            cache_dir,
        }
    }

    pub fn from_build_cache(cache: &BuildCache, cache_dir: PathBuf) -> Self {
        // Create cache directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            debug!("Failed to create cache directory: {}", e);
        }

        let placeholders = cache
            .image_placeholders
            .iter()
            .map(|(path, thumbhash)| {
                (
                    path.clone(),
                    PlaceholderCacheEntry {
                        thumbhash: thumbhash.clone(),
                    },
                )
            })
            .collect();

        let transformed = cache
            .image_transformed
            .iter()
            .map(|(key, cached_path)| {
                (
                    key.clone(),
                    TransformedImageCacheEntry {
                        cached_path: cached_path.clone(),
                    },
                )
            })
            .collect();

        debug!(
            "Image cache initialized from build cache with {} placeholders and {} transformed images",
            cache.image_placeholders.len(),
            cache.image_transformed.len()
        );

        Self {
            placeholders,
            transformed,
            cache_dir,
        }
    }

    pub fn write_to_build_cache(&self, cache: &mut BuildCache) {
        cache.image_placeholders = self
            .placeholders
            .iter()
            .map(|(path, entry)| (path.clone(), entry.thumbhash.clone()))
            .collect();

        cache.image_transformed = self
            .transformed
            .iter()
            .map(|(key, entry)| (key.clone(), entry.cached_path.clone()))
            .collect();
    }

    /// Get cached placeholder or None if not found
    pub fn get_placeholder(&self, src_path: &Path) -> Option<PlaceholderCacheEntry> {
        let entry = self.placeholders.get(src_path)?;

        debug!("Placeholder cache hit for {}", src_path.display());
        Some(entry.clone())
    }

    /// Cache a placeholder
    pub fn cache_placeholder(&mut self, src_path: &Path, thumbhash: Vec<u8>) {
        let entry = PlaceholderCacheEntry { thumbhash };

        self.placeholders.insert(src_path.to_path_buf(), entry);
        debug!("Cached placeholder for {}", src_path.display());
    }

    /// Get cached transformed image path or None if not found
    pub fn get_transformed_image(&self, final_filename: &Path) -> Option<PathBuf> {
        let entry = self.transformed.get(final_filename)?;

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

        self.transformed
            .insert(final_filename.to_path_buf(), entry);
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

pub const DEFAULT_IMAGE_CACHE_DIR: &str = "target/maudit_cache/images";

impl ImageCache {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::new(
            PathBuf::from(DEFAULT_IMAGE_CACHE_DIR),
        ))))
    }

    pub fn with_cache_dir<P: AsRef<Path>>(cache_dir_path: P) -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::new(
            cache_dir_path.as_ref().to_path_buf(),
        ))))
    }

    pub fn from_build_cache<P: AsRef<Path>>(cache: &BuildCache, cache_dir_path: P) -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::from_build_cache(
            cache,
            cache_dir_path.as_ref().to_path_buf(),
        ))))
    }

    pub fn write_to_build_cache(&self, cache: &mut BuildCache) {
        self.lock_inner().write_to_build_cache(cache);
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

    #[test]
    fn test_build_cache_roundtrip() {
        use crate::build::cache::BUILD_CACHE_VERSION;

        let image_cache = ImageCache::new();
        image_cache.cache_placeholder(Path::new("test.png"), vec![10, 20, 30]);
        image_cache.cache_transformed_image(
            Path::new("test.abc123.webp"),
            PathBuf::from("/tmp/cached/test.abc123.webp"),
        );

        // Write to build cache
        let mut build_cache = BuildCache {
            version: BUILD_CACHE_VERSION,
            ..Default::default()
        };
        image_cache.write_to_build_cache(&mut build_cache);

        // Verify data is in build cache
        assert_eq!(
            build_cache.image_placeholders.get(Path::new("test.png")),
            Some(&vec![10u8, 20, 30])
        );
        assert_eq!(
            build_cache
                .image_transformed
                .get(Path::new("test.abc123.webp")),
            Some(&PathBuf::from("/tmp/cached/test.abc123.webp"))
        );

        // Reconstruct from build cache
        let restored = ImageCache::from_build_cache(&build_cache, PathBuf::from(DEFAULT_IMAGE_CACHE_DIR));

        let placeholder = restored.get_placeholder(Path::new("test.png"));
        assert!(placeholder.is_some());
        assert_eq!(placeholder.unwrap().thumbhash, vec![10, 20, 30]);
    }
}
