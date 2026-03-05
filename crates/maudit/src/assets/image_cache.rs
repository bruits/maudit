use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use log::debug;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

const IMAGE_CACHE_FILENAME: &str = "image_cache.bin";

/// Serializable image cache data, persisted independently of the build cache.
#[derive(Serialize, Deserialize, Default)]
struct PersistedImageCache {
    placeholders: FxHashMap<PathBuf, (Vec<u8>, String)>,
    transformed: FxHashMap<PathBuf, PathBuf>,
}

#[derive(Debug, Clone)]
pub struct PlaceholderCacheEntry {
    pub thumbhash: Vec<u8>,
    /// Hash of the source image content, used to invalidate stale entries.
    pub source_hash: String,
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
        Self {
            placeholders: FxHashMap::default(),
            transformed: FxHashMap::default(),
            cache_dir,
        }
    }

    pub fn load(cache_dir: PathBuf, persisted_dir: &Path) -> Self {
        let path = persisted_dir.join(IMAGE_CACHE_FILENAME);
        let persisted = fs::read(&path).ok().and_then(|bytes| {
            bincode::deserialize::<PersistedImageCache>(&bytes)
                .map_err(|e| debug!("Failed to deserialize image cache: {}", e))
                .ok()
        });

        let Some(persisted) = persisted else {
            return Self::new(cache_dir);
        };

        debug!(
            "Image cache loaded with {} placeholders and {} transformed images",
            persisted.placeholders.len(),
            persisted.transformed.len()
        );

        let placeholders = persisted
            .placeholders
            .into_iter()
            .map(|(path, (thumbhash, source_hash))| {
                (
                    path,
                    PlaceholderCacheEntry {
                        thumbhash,
                        source_hash,
                    },
                )
            })
            .collect();

        let transformed = persisted
            .transformed
            .into_iter()
            .map(|(key, cached_path)| (key, TransformedImageCacheEntry { cached_path }))
            .collect();

        Self {
            placeholders,
            transformed,
            cache_dir,
        }
    }

    pub fn save(&self, persisted_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(persisted_dir)?;
        let path = persisted_dir.join(IMAGE_CACHE_FILENAME);
        let tmp_path = persisted_dir.join(format!("{}.tmp", IMAGE_CACHE_FILENAME));

        let persisted = PersistedImageCache {
            placeholders: self
                .placeholders
                .iter()
                .map(|(path, entry)| {
                    (
                        path.clone(),
                        (entry.thumbhash.clone(), entry.source_hash.clone()),
                    )
                })
                .collect(),
            transformed: self
                .transformed
                .iter()
                .map(|(key, entry)| (key.clone(), entry.cached_path.clone()))
                .collect(),
        };

        let bytes =
            bincode::serialize(&persisted).expect("ImageCache serialization should not fail");
        fs::write(&tmp_path, bytes)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    /// Get cached placeholder or None if not found.
    /// Returns None if the source hash doesn't match (image was modified).
    pub fn get_placeholder(
        &self,
        src_path: &Path,
        source_hash: &str,
    ) -> Option<PlaceholderCacheEntry> {
        let entry = self.placeholders.get(src_path)?;

        if entry.source_hash != source_hash {
            debug!(
                "Placeholder cache stale for {} (hash mismatch)",
                src_path.display()
            );
            return None;
        }

        debug!("Placeholder cache hit for {}", src_path.display());
        Some(entry.clone())
    }

    /// Cache a placeholder
    pub fn cache_placeholder(&mut self, src_path: &Path, thumbhash: Vec<u8>, source_hash: String) {
        let entry = PlaceholderCacheEntry {
            thumbhash,
            source_hash,
        };

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

        self.transformed.insert(final_filename.to_path_buf(), entry);
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

    /// Generate a cache path for a transformed image, creating the cache
    /// directory if it doesn't exist yet.
    pub fn generate_cache_path(&self, final_filename: &Path) -> PathBuf {
        let _ = fs::create_dir_all(&self.cache_dir);
        self.cache_dir.join(final_filename)
    }

    /// Remove placeholder and transformed entries not in the given sets.
    /// Also deletes orphaned files from the cache directory.
    pub fn gc(
        &mut self,
        live_placeholder_paths: &FxHashSet<PathBuf>,
        live_transformed_filenames: &FxHashSet<PathBuf>,
    ) -> usize {
        let before = self.placeholders.len() + self.transformed.len();

        self.placeholders
            .retain(|k, _| live_placeholder_paths.contains(k));

        let mut orphaned_files = Vec::new();
        self.transformed.retain(|k, entry| {
            if live_transformed_filenames.contains(k) {
                true
            } else {
                orphaned_files.push(entry.cached_path.clone());
                false
            }
        });

        // Clean up orphaned cached files on disk
        for path in &orphaned_files {
            if let Err(e) = fs::remove_file(path) {
                debug!(
                    "Failed to remove orphaned cache file {}: {}",
                    path.display(),
                    e
                );
            }
        }

        let after = self.placeholders.len() + self.transformed.len();
        before - after
    }
}

impl ImageCache {
    pub fn new() -> Self {
        Self::with_cache_dir("target/maudit/images")
    }

    pub fn with_cache_dir<P: AsRef<Path>>(cache_dir_path: P) -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::new(
            cache_dir_path.as_ref().to_path_buf(),
        ))))
    }

    /// Load image cache from its own persisted file, independent of the build cache.
    pub fn load<P: AsRef<Path>>(cache_dir_path: P, persisted_dir: &Path) -> Self {
        Self(Arc::new(Mutex::new(ImageCacheInner::load(
            cache_dir_path.as_ref().to_path_buf(),
            persisted_dir,
        ))))
    }

    /// Save image cache to its own file, independent of the build cache.
    pub fn save(&self, persisted_dir: &Path) -> std::io::Result<()> {
        self.lock_inner().save(persisted_dir)
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

    /// Get cached placeholder or None if not found.
    /// Returns None if the source hash doesn't match (image was modified).
    pub fn get_placeholder(
        &self,
        src_path: &Path,
        source_hash: &str,
    ) -> Option<PlaceholderCacheEntry> {
        self.lock_inner().get_placeholder(src_path, source_hash)
    }

    /// Cache a placeholder
    pub fn cache_placeholder(&self, src_path: &Path, thumbhash: Vec<u8>, source_hash: String) {
        self.lock_inner()
            .cache_placeholder(src_path, thumbhash, source_hash)
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

    /// Returns true if the cache has no entries.
    pub fn is_empty(&self) -> bool {
        let inner = self.lock_inner();
        inner.placeholders.is_empty() && inner.transformed.is_empty()
    }

    /// Get the cache directory path
    pub fn get_cache_dir(&self) -> PathBuf {
        self.lock_inner().get_cache_dir().clone()
    }

    /// Generate a cache path for a transformed image
    pub fn generate_cache_path(&self, final_filename: &Path) -> PathBuf {
        self.lock_inner().generate_cache_path(final_filename)
    }

    /// Remove entries not referenced by any current page.
    /// `live_placeholder_paths`: source paths of images used in the current build.
    /// `live_transformed_filenames`: final filenames of transformed images in the current build.
    /// Returns the number of evicted entries.
    pub fn gc(
        &self,
        live_placeholder_paths: &FxHashSet<PathBuf>,
        live_transformed_filenames: &FxHashSet<PathBuf>,
    ) -> usize {
        self.lock_inner()
            .gc(live_placeholder_paths, live_transformed_filenames)
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
        let cache = ImageCache::new();
        assert_eq!(cache.get_cache_dir(), PathBuf::from("target/maudit/images"));
    }

    #[test]
    fn test_build_options_integration() {
        use crate::build::options::BuildOptions;

        // Test that image cache dir is derived from cache_dir
        let build_options = BuildOptions {
            cache_dir: PathBuf::from("/tmp/custom_maudit_cache"),
            ..Default::default()
        };

        let image_cache_dir = build_options.cache_dir.join("images");
        let cache = ImageCache::with_cache_dir(&image_cache_dir);

        assert_eq!(
            cache.get_cache_dir(),
            PathBuf::from("/tmp/custom_maudit_cache/images")
        );
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let cache = ImageCache::new();
        let cache_clone = cache.clone();

        // Test that the cache can be shared across threads
        let handle = thread::spawn(move || {
            cache_clone.cache_placeholder(
                Path::new("test.jpg"),
                vec![1, 2, 3, 4],
                "hash1".to_string(),
            );
        });

        handle.join().unwrap();

        // Verify the placeholder was cached
        let entry = cache.get_placeholder(Path::new("test.jpg"), "hash1");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().thumbhash, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("images");
        let persisted_dir = dir.path().join("cache");

        let image_cache = ImageCache::with_cache_dir(&cache_dir);
        image_cache.cache_placeholder(
            Path::new("test.png"),
            vec![10, 20, 30],
            "srchash".to_string(),
        );
        image_cache.cache_transformed_image(
            Path::new("test.abc123.webp"),
            PathBuf::from("/tmp/cached/test.abc123.webp"),
        );

        // Save to its own file
        image_cache.save(&persisted_dir).unwrap();

        // Load from file
        let restored = ImageCache::load(&cache_dir, &persisted_dir);

        // Matching hash returns the entry
        let placeholder = restored.get_placeholder(Path::new("test.png"), "srchash");
        assert!(placeholder.is_some());
        assert_eq!(placeholder.unwrap().thumbhash, vec![10, 20, 30]);

        // Mismatched hash returns None (stale entry)
        let stale = restored.get_placeholder(Path::new("test.png"), "different_hash");
        assert!(stale.is_none());
    }

    #[test]
    fn test_load_missing_file_returns_empty_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("images");
        let persisted_dir = dir.path().join("nonexistent");

        let cache = ImageCache::load(&cache_dir, &persisted_dir);
        assert!(
            cache
                .get_placeholder(Path::new("anything"), "hash")
                .is_none()
        );
    }

    #[test]
    fn test_gc_evicts_stale_entries() {
        let temp_dir = env::temp_dir().join("test_maudit_gc");
        let cache = ImageCache::with_cache_dir(&temp_dir);

        // Add some placeholders
        cache.cache_placeholder(Path::new("/img/a.png"), vec![1], "ha".to_string());
        cache.cache_placeholder(Path::new("/img/b.png"), vec![2], "hb".to_string());
        cache.cache_placeholder(Path::new("/img/c.png"), vec![3], "hc".to_string());

        // Add some transformed images
        cache.cache_transformed_image(Path::new("a.abc.webp"), temp_dir.join("a.abc.webp"));
        cache.cache_transformed_image(Path::new("b.def.webp"), temp_dir.join("b.def.webp"));

        // Only a.png and a.abc.webp are still live
        let live_placeholders: FxHashSet<PathBuf> =
            [PathBuf::from("/img/a.png")].into_iter().collect();
        let live_transformed: FxHashSet<PathBuf> =
            [PathBuf::from("a.abc.webp")].into_iter().collect();

        let evicted = cache.gc(&live_placeholders, &live_transformed);
        assert_eq!(evicted, 3); // b.png, c.png placeholders + b.def.webp transformed

        // a.png still accessible
        assert!(
            cache
                .get_placeholder(Path::new("/img/a.png"), "ha")
                .is_some()
        );
        // b.png evicted
        assert!(
            cache
                .get_placeholder(Path::new("/img/b.png"), "hb")
                .is_none()
        );
        // a.abc.webp still accessible (though file doesn't exist, it's still in the map)
        assert!(
            cache
                .get_transformed_image(Path::new("a.abc.webp"))
                .is_none()
        ); // file doesn't exist
        // b.def.webp evicted
        assert!(
            cache
                .get_transformed_image(Path::new("b.def.webp"))
                .is_none()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_placeholder_invalidation_on_source_change() {
        let cache = ImageCache::new();

        // Cache a placeholder with hash "v1"
        cache.cache_placeholder(Path::new("img.png"), vec![1, 2, 3], "v1".to_string());

        // Same hash → hit
        assert!(cache.get_placeholder(Path::new("img.png"), "v1").is_some());

        // Different hash (source changed) → miss
        assert!(cache.get_placeholder(Path::new("img.png"), "v2").is_none());

        // Cache with new hash
        cache.cache_placeholder(Path::new("img.png"), vec![4, 5, 6], "v2".to_string());

        // New hash → hit with new data
        let entry = cache.get_placeholder(Path::new("img.png"), "v2").unwrap();
        assert_eq!(entry.thumbhash, vec![4, 5, 6]);

        // Old hash → miss
        assert!(cache.get_placeholder(Path::new("img.png"), "v1").is_none());
    }
}
