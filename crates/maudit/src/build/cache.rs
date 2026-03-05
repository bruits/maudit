use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use log::{debug, info};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

pub const BUILD_CACHE_VERSION: u32 = 4;
pub const BUILD_CACHE_FILENAME: &str = "build_cache.bin";

/// Fingerprint for an asset file (script, style, image) used for fast change detection.
/// On incremental rebuilds, we stat the file first; if mtime+size match the cached
/// values, we skip re-hashing.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssetFileFingerprint {
    pub hash: String,
    pub mtime_ns: u64,
    pub size: u64,
}

impl AssetFileFingerprint {
    pub fn from_path(path: &Path) -> Option<Self> {
        let hash = hash_file_content(path)?;
        let (mtime_ns, size) = file_fingerprint(path)?;
        Some(Self {
            hash,
            mtime_ns,
            size,
        })
    }
}

/// The full build cache, persisted to disk between builds.
#[derive(Serialize, Deserialize, Default)]
pub struct BuildCache {
    pub version: u32,
    /// Hash of the running executable binary (mtime + size).
    /// If this changes, the entire cache is invalidated.
    pub binary_hash: String,
    /// Per content-source state: source_name -> file hashes + entry IDs.
    pub content_sources: FxHashMap<String, ContentSourceState>,
    /// Per-page dependency information.
    pub pages: FxHashMap<PageKey, PageCacheEntry>,
    /// Per-route-pattern: info from the pages() call for dynamic routes.
    pub route_pages: FxHashMap<String, RoutePagesInfo>,
    /// Fingerprints of asset files (scripts, styles, images) used across the build.
    pub asset_file_hashes: FxHashMap<PathBuf, AssetFileFingerprint>,
    /// The set of bundled script inputs from the last build.
    pub bundled_scripts: Vec<SerializedAssetRef>,
    /// The set of bundled style inputs from the last build.
    pub bundled_styles: Vec<SerializedAssetRef>,
    /// Cached image placeholder thumbhashes (src_path → thumbhash bytes).
    pub image_placeholders: FxHashMap<PathBuf, Vec<u8>>,
    /// Cached transformed image paths (final_filename → cached_file_path).
    pub image_transformed: FxHashMap<PathBuf, PathBuf>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ContentSourceState {
    /// Mapping from file_path to content hash for each entry.
    pub files: FxHashMap<PathBuf, String>,
    /// Sorted list of entry IDs — used to detect structural changes.
    pub entry_ids: Vec<String>,
    /// Reverse map from file_path to entry_id. Not serialized — rebuilt each run.
    #[serde(skip)]
    pub file_to_entry: FxHashMap<PathBuf, String>,
}

/// Canonical key for a generated page. Must be stable across builds.
///
/// Uses a sorted Vec for params instead of FxHashMap to support Hash + deterministic comparison.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageKey {
    pub route: String,
    /// Sorted by key for deterministic equality and hashing.
    pub params: Vec<(String, Option<String>)>,
    pub variant: Option<String>,
}

impl PageKey {
    pub fn new(
        route: &str,
        params: &FxHashMap<String, Option<String>>,
        variant: Option<&str>,
    ) -> Self {
        let mut sorted_params: Vec<(String, Option<String>)> =
            params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        sorted_params.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            route: route.to_string(),
            params: sorted_params,
            variant: variant.map(|s| s.to_string()),
        }
    }

    pub fn new_static(route: &str, variant: Option<&str>) -> Self {
        Self {
            route: route.to_string(),
            params: Vec::new(),
            variant: variant.map(|s| s.to_string()),
        }
    }
}

/// Cached dependencies for a single generated page.
#[derive(Serialize, Deserialize, Clone)]
pub struct PageCacheEntry {
    /// Content entries this page read via get_entry().
    pub content_entries_read: Vec<(String, String)>,
    /// Content sources this page fully iterated.
    pub content_sources_iterated: Vec<String>,
    /// Image assets used by this page.
    pub images: Vec<CachedImage>,
    /// Script assets used by this page.
    pub scripts: Vec<CachedScript>,
    /// Style assets used by this page.
    pub styles: Vec<CachedStyle>,
    /// The output file path (relative to cwd).
    pub output_file: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CachedScript {
    pub path: PathBuf,
    pub hash: String,
    pub included: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CachedStyle {
    pub path: PathBuf,
    pub hash: String,
    pub included: bool,
    pub tailwind: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CachedImage {
    pub path: PathBuf,
    pub hash: String,
}

/// Info from a dynamic route's pages() call.
#[derive(Serialize, Deserialize, Clone)]
pub struct RoutePagesInfo {
    /// Content sources accessed during pages().
    pub sources_accessed: Vec<String>,
    /// The resulting page keys.
    pub page_keys: Vec<PageKey>,
}

/// A serializable reference to an asset file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SerializedAssetRef {
    pub path: PathBuf,
    pub hash: String,
}

impl BuildCache {
    pub fn load(cache_dir: &Path) -> Option<Self> {
        let path = cache_dir.join(BUILD_CACHE_FILENAME);
        let bytes = fs::read(&path).ok()?;
        let cache: Self = match bincode::deserialize(&bytes) {
            Ok(c) => c,
            Err(e) => {
                debug!("Failed to deserialize build cache: {}", e);
                return None;
            }
        };

        if cache.version != BUILD_CACHE_VERSION {
            debug!(
                "Build cache version mismatch: found {}, expected {}",
                cache.version, BUILD_CACHE_VERSION
            );
            return None;
        }

        Some(cache)
    }

    pub fn save(&self, cache_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(cache_dir)?;
        let path = cache_dir.join(BUILD_CACHE_FILENAME);
        let tmp_path = cache_dir.join(format!("{}.tmp", BUILD_CACHE_FILENAME));
        let bytes = bincode::serialize(self).expect("BuildCache serialization should not fail");
        fs::write(&tmp_path, bytes)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    /// Compute a fast hash of the current executable (mtime + size).
    pub fn compute_binary_hash() -> String {
        let Ok(exe_path) = std::env::current_exe() else {
            return String::new();
        };
        let Ok(metadata) = fs::metadata(&exe_path) else {
            return String::new();
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let size = metadata.len();
        format!("{:x}-{:x}", modified, size)
    }
}

/// Result of diffing the current state against the cached state.
pub struct IncrementalState {
    pub mode: IncrementalMode,
    pub previous_cache: Option<BuildCache>,
    /// Set of PageKeys that need re-rendering.
    pub dirty_pages: FxHashSet<PageKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrementalMode {
    Full,
    Incremental,
}

impl IncrementalState {
    /// Create state for a full (non-incremental) build.
    pub fn full_build() -> Self {
        Self {
            mode: IncrementalMode::Full,
            previous_cache: None,
            dirty_pages: FxHashSet::default(),
        }
    }

    /// Returns true if every page should be rendered (full build or no cache).
    pub fn is_full_build(&self) -> bool {
        self.mode == IncrementalMode::Full
    }

    /// Check if a specific page needs re-rendering.
    pub fn is_page_dirty(&self, key: &PageKey) -> bool {
        match self.mode {
            IncrementalMode::Full => true,
            IncrementalMode::Incremental => self.dirty_pages.contains(key),
        }
    }
}

/// Hash raw bytes using rapidhash for speed.
pub fn hash_bytes(content: &[u8]) -> String {
    use rapidhash::fast::RapidHasher;
    use std::hash::Hasher;

    let mut hasher = RapidHasher::default();
    hasher.write(content);
    format!("{:016x}", hasher.finish())
}

/// Hash file content using rapidhash for speed.
pub fn hash_file_content(path: &Path) -> Option<String> {
    let content = fs::read(path).ok()?;
    Some(hash_bytes(&content))
}

/// Compute the current ContentSourceState for a single content source.
///
/// `raw_content_by_entry` maps entry IDs to their already-loaded raw content.
/// When an entry has exactly one file dep and its content is available, we hash
/// from memory instead of re-reading from disk.
pub fn compute_content_source_state(
    entry_file_info: &[(String, Vec<PathBuf>)],
    raw_content_by_entry: &FxHashMap<String, &str>,
) -> ContentSourceState {
    let mut files = FxHashMap::default();
    let mut entry_ids: Vec<String> = Vec::with_capacity(entry_file_info.len());
    let mut file_to_entry = FxHashMap::default();

    for (id, file_paths) in entry_file_info {
        entry_ids.push(id.clone());

        // If the entry has exactly one file dep and we have its raw_content, hash from memory
        let in_memory_hash = if file_paths.len() == 1 {
            raw_content_by_entry
                .get(id.as_str())
                .map(|content| hash_bytes(content.as_bytes()))
        } else {
            None
        };

        for (i, fp) in file_paths.iter().enumerate() {
            let hash = if i == 0 { in_memory_hash.clone() } else { None }
                .or_else(|| hash_file_content(fp));

            if let Some(hash) = hash {
                files.insert(fp.clone(), hash);
                file_to_entry.insert(fp.clone(), id.clone());
            }
        }
    }

    entry_ids.sort();

    ContentSourceState {
        files,
        entry_ids,
        file_to_entry,
    }
}

/// Diff content sources between cached and current state.
/// Returns (structurally_changed_sources, changed_entries).
pub fn diff_content_sources(
    cached: &FxHashMap<String, ContentSourceState>,
    current: &FxHashMap<String, ContentSourceState>,
) -> (FxHashSet<String>, FxHashSet<(String, String)>) {
    let mut structurally_changed = FxHashSet::default();
    let mut changed_entries = FxHashSet::default();

    for (name, current_state) in current {
        match cached.get(name) {
            None => {
                // Entirely new source — structural change
                structurally_changed.insert(name.clone());
            }
            Some(cached_state) => {
                // Check structural change (entry IDs differ)
                if cached_state.entry_ids != current_state.entry_ids {
                    structurally_changed.insert(name.clone());
                }

                // Check per-file content changes
                for (file_path, current_hash) in &current_state.files {
                    match cached_state.files.get(file_path) {
                        Some(cached_hash) if cached_hash == current_hash => {
                            // Unchanged
                        }
                        _ => {
                            // File changed or new — look up the owning entry via reverse map
                            if let Some(entry_id) = current_state.file_to_entry.get(file_path) {
                                changed_entries.insert((name.clone(), entry_id.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for removed sources
    for name in cached.keys() {
        if !current.contains_key(name) {
            structurally_changed.insert(name.clone());
        }
    }

    (structurally_changed, changed_entries)
}

/// Get file mtime (nanoseconds since epoch) and size for fast change detection.
pub fn file_fingerprint(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos() as u64;
    Some((mtime, meta.len()))
}

/// Diff asset files using mtime+size fingerprints for fast filtering.
/// Only re-hashes files whose metadata changed.
pub fn diff_asset_files(cached: &FxHashMap<PathBuf, AssetFileFingerprint>) -> FxHashSet<PathBuf> {
    let mut changed = FxHashSet::default();
    for (path, cached_fp) in cached {
        match file_fingerprint(path) {
            Some((mtime, size)) if mtime == cached_fp.mtime_ns && size == cached_fp.size => {
                // mtime+size match → assume unchanged
            }
            Some(_) => {
                // Metadata changed → re-hash to confirm (handles touch without content change)
                match hash_file_content(path) {
                    Some(hash) if hash == cached_fp.hash => {}
                    _ => {
                        changed.insert(path.clone());
                    }
                }
            }
            None => {
                // File missing or unreadable
                changed.insert(path.clone());
            }
        }
    }
    changed
}

/// Determine which pages are dirty based on content and asset changes.
pub fn determine_dirty_pages(
    cache: &BuildCache,
    structurally_changed_sources: &FxHashSet<String>,
    changed_entries: &FxHashSet<(String, String)>,
    changed_asset_files: &FxHashSet<PathBuf>,
) -> FxHashSet<PageKey> {
    let mut dirty = FxHashSet::default();

    for (page_key, page_entry) in &cache.pages {
        let mut is_dirty = false;

        // Check if any iterated source changed structurally
        for source_name in &page_entry.content_sources_iterated {
            if structurally_changed_sources.contains(source_name) {
                is_dirty = true;
                break;
            }
        }

        if !is_dirty {
            // Check if any iterated source had any content change at all
            // (since the page iterates all entries, any change affects it)
            for source_name in &page_entry.content_sources_iterated {
                if changed_entries.iter().any(|(s, _)| s == source_name) {
                    is_dirty = true;
                    break;
                }
            }
        }

        if !is_dirty {
            // Check if any specifically-read entry changed
            for (source_name, entry_id) in &page_entry.content_entries_read {
                if changed_entries.contains(&(source_name.clone(), entry_id.clone())) {
                    is_dirty = true;
                    break;
                }
            }
        }

        if !is_dirty {
            // Check if any asset file used by this page changed
            let page_asset_paths = page_entry
                .images
                .iter()
                .map(|a| &a.path)
                .chain(page_entry.scripts.iter().map(|a| &a.path))
                .chain(page_entry.styles.iter().map(|a| &a.path));

            for path in page_asset_paths {
                if changed_asset_files.contains(path) {
                    is_dirty = true;
                    break;
                }
            }
        }

        if is_dirty {
            dirty.insert(page_key.clone());
        }
    }

    dirty
}

/// Determine which cached pages are stale (no longer generated).
pub fn find_stale_pages(
    cached_pages: &FxHashMap<PageKey, PageCacheEntry>,
    current_pages: &FxHashSet<PageKey>,
) -> FxHashSet<PageKey> {
    cached_pages
        .keys()
        .filter(|k| !current_pages.contains(k))
        .cloned()
        .collect()
}

/// Check whether rebundling is needed by comparing asset sets.
pub fn needs_rebundle(
    cached_scripts: &[SerializedAssetRef],
    cached_styles: &[SerializedAssetRef],
    current_scripts: &FxHashSet<SerializedAssetRef>,
    current_styles: &FxHashSet<SerializedAssetRef>,
) -> bool {
    let cached_scripts_set: FxHashSet<&SerializedAssetRef> = cached_scripts.iter().collect();
    let cached_styles_set: FxHashSet<&SerializedAssetRef> = cached_styles.iter().collect();

    let current_scripts_ref: FxHashSet<&SerializedAssetRef> = current_scripts.iter().collect();
    let current_styles_ref: FxHashSet<&SerializedAssetRef> = current_styles.iter().collect();

    cached_scripts_set != current_scripts_ref || cached_styles_set != current_styles_ref
}

/// Compute incremental state from a previously loaded cache and current content.
///
/// `current_content_states` should be pre-computed via `compute_content_source_state()`
/// for each content source. `current_binary_hash` is the hash of the running executable.
pub fn load_incremental_state(
    previous_cache: Option<BuildCache>,
    current_content_states: &FxHashMap<String, ContentSourceState>,
    current_binary_hash: &str,
) -> IncrementalState {
    let cache = match previous_cache {
        Some(c) => c,
        None => {
            info!(target: "cache", "No valid build cache found, performing full build");
            return IncrementalState::full_build();
        }
    };

    // Check binary hash
    if cache.binary_hash != current_binary_hash {
        info!(target: "cache", "Binary changed, performing full build");
        return IncrementalState::full_build();
    }

    // Diff content sources
    let (structurally_changed_sources, changed_entries) =
        diff_content_sources(&cache.content_sources, current_content_states);

    if !structurally_changed_sources.is_empty() {
        info!(
            target: "cache",
            "Content sources with structural changes: {:?}",
            structurally_changed_sources
        );
    }

    if !changed_entries.is_empty() {
        info!(
            target: "cache",
            "Changed content entries: {:?}",
            changed_entries
        );
    }

    // Diff asset files (scripts, styles, images) against cached hashes
    let changed_asset_files = diff_asset_files(&cache.asset_file_hashes);

    if !changed_asset_files.is_empty() {
        info!(
            target: "cache",
            "Changed asset files: {:?}",
            changed_asset_files
        );
    }

    // Determine dirty pages from content + asset changes
    let dirty_pages = determine_dirty_pages(
        &cache,
        &structurally_changed_sources,
        &changed_entries,
        &changed_asset_files,
    );

    IncrementalState {
        mode: IncrementalMode::Incremental,
        previous_cache: Some(cache),
        dirty_pages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_entry(
        entries_read: Vec<(String, String)>,
        sources_iterated: Vec<String>,
        output_file: &str,
    ) -> PageCacheEntry {
        PageCacheEntry {
            content_entries_read: entries_read,
            content_sources_iterated: sources_iterated,
            images: vec![],
            scripts: vec![],
            styles: vec![],
            output_file: PathBuf::from(output_file),
        }
    }

    #[test]
    fn test_page_key_equality() {
        let key1 = PageKey::new_static("/", None);
        let key2 = PageKey::new_static("/", None);
        assert_eq!(key1, key2);

        let key3 = PageKey::new_static("/", Some("en"));
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_roundtrip() {
        let cache = BuildCache {
            version: BUILD_CACHE_VERSION,
            binary_hash: "abc123".to_string(),
            ..Default::default()
        };

        let bytes = bincode::serialize(&cache).unwrap();
        let loaded: BuildCache = bincode::deserialize(&bytes).unwrap();

        assert_eq!(loaded.version, BUILD_CACHE_VERSION);
        assert_eq!(loaded.binary_hash, "abc123");
    }

    #[test]
    fn test_diff_content_sources_no_changes() {
        let mut cached = FxHashMap::default();
        cached.insert(
            "articles".to_string(),
            ContentSourceState {
                files: {
                    let mut m = FxHashMap::default();
                    m.insert(PathBuf::from("content/a.md"), "hash1".to_string());
                    m
                },
                entry_ids: vec!["a".to_string()],
                ..Default::default()
            },
        );

        let current = cached.clone();
        let (structural, changed) = diff_content_sources(&cached, &current);

        assert!(structural.is_empty());
        assert!(changed.is_empty());
    }

    #[test]
    fn test_diff_content_sources_entry_changed() {
        let mut cached = FxHashMap::default();
        cached.insert(
            "articles".to_string(),
            ContentSourceState {
                files: {
                    let mut m = FxHashMap::default();
                    m.insert(PathBuf::from("content/a.md"), "hash1".to_string());
                    m
                },
                entry_ids: vec!["a".to_string()],
                ..Default::default()
            },
        );

        let mut current = FxHashMap::default();
        current.insert(
            "articles".to_string(),
            ContentSourceState {
                files: {
                    let mut m = FxHashMap::default();
                    m.insert(PathBuf::from("content/a.md"), "hash2".to_string());
                    m
                },
                entry_ids: vec!["a".to_string()],
                file_to_entry: {
                    let mut m = FxHashMap::default();
                    m.insert(PathBuf::from("content/a.md"), "a".to_string());
                    m
                },
            },
        );

        let (structural, changed) = diff_content_sources(&cached, &current);

        assert!(structural.is_empty());
        assert!(changed.contains(&("articles".to_string(), "a".to_string())));
    }

    #[test]
    fn test_diff_content_sources_structural_change() {
        let mut cached = FxHashMap::default();
        cached.insert(
            "articles".to_string(),
            ContentSourceState {
                files: FxHashMap::default(),
                entry_ids: vec!["a".to_string()],
                ..Default::default()
            },
        );

        let mut current = FxHashMap::default();
        current.insert(
            "articles".to_string(),
            ContentSourceState {
                files: FxHashMap::default(),
                entry_ids: vec!["a".to_string(), "b".to_string()],
                ..Default::default()
            },
        );

        let (structural, _changed) = diff_content_sources(&cached, &current);
        assert!(structural.contains("articles"));
    }

    #[test]
    fn test_determine_dirty_pages() {
        let mut pages = FxHashMap::default();
        let key_index = PageKey::new_static("/", None);
        pages.insert(
            key_index.clone(),
            page_entry(vec![], vec!["articles".to_string()], "dist/index.html"),
        );

        let key_article = PageKey::new(
            "/articles/[slug]",
            &{
                let mut m = FxHashMap::default();
                m.insert("slug".to_string(), Some("foo".to_string()));
                m
            },
            None,
        );
        pages.insert(
            key_article.clone(),
            page_entry(
                vec![("articles".to_string(), "foo".to_string())],
                vec![],
                "dist/articles/foo/index.html",
            ),
        );

        let key_other = PageKey::new(
            "/articles/[slug]",
            &{
                let mut m = FxHashMap::default();
                m.insert("slug".to_string(), Some("bar".to_string()));
                m
            },
            None,
        );
        pages.insert(
            key_other.clone(),
            page_entry(
                vec![("articles".to_string(), "bar".to_string())],
                vec![],
                "dist/articles/bar/index.html",
            ),
        );

        let cache = BuildCache {
            pages,
            ..Default::default()
        };

        // Only "foo" entry changed
        let mut changed_entries = FxHashSet::default();
        changed_entries.insert(("articles".to_string(), "foo".to_string()));

        let dirty = determine_dirty_pages(
            &cache,
            &FxHashSet::default(),
            &changed_entries,
            &FxHashSet::default(),
        );

        // Index is dirty because it iterates "articles"
        assert!(dirty.contains(&key_index));
        // "foo" article is dirty because it reads entry "foo"
        assert!(dirty.contains(&key_article));
        // "bar" article is NOT dirty
        assert!(!dirty.contains(&key_other));
    }

    #[test]
    fn test_needs_rebundle() {
        let scripts = vec![SerializedAssetRef {
            path: PathBuf::from("script.js"),
            hash: "abc".to_string(),
        }];
        let styles = vec![];

        let current_scripts: FxHashSet<SerializedAssetRef> = scripts.iter().cloned().collect();
        let current_styles: FxHashSet<SerializedAssetRef> = FxHashSet::default();

        assert!(!needs_rebundle(
            &scripts,
            &styles,
            &current_scripts,
            &current_styles
        ));

        // Add a new script
        let mut new_scripts = current_scripts;
        new_scripts.insert(SerializedAssetRef {
            path: PathBuf::from("new.js"),
            hash: "def".to_string(),
        });

        assert!(needs_rebundle(
            &scripts,
            &styles,
            &new_scripts,
            &current_styles
        ));
    }

    #[test]
    fn test_page_no_deps_stays_clean() {
        // A page with no content dependencies (e.g. an "about" page)
        // should stay clean when content changes.
        let mut pages = FxHashMap::default();
        let key = PageKey::new_static("/about", None);
        pages.insert(
            key.clone(),
            page_entry(vec![], vec![], "dist/about/index.html"),
        );

        let cache = BuildCache {
            pages,
            ..Default::default()
        };

        let mut changed_entries = FxHashSet::default();
        changed_entries.insert(("articles".to_string(), "foo".to_string()));

        let dirty = determine_dirty_pages(
            &cache,
            &FxHashSet::default(),
            &changed_entries,
            &FxHashSet::default(),
        );

        assert!(!dirty.contains(&key));
    }

    #[test]
    fn test_determine_dirty_pages_asset_change() {
        let mut pages = FxHashMap::default();
        let key = PageKey::new_static("/", None);
        pages.insert(
            key.clone(),
            PageCacheEntry {
                images: vec![CachedImage {
                    path: PathBuf::from("images/logo.png"),
                    hash: "img_hash".to_string(),
                }],
                ..page_entry(vec![], vec![], "dist/index.html")
            },
        );

        let cache = BuildCache {
            pages,
            ..Default::default()
        };

        let mut changed_assets = FxHashSet::default();
        changed_assets.insert(PathBuf::from("images/logo.png"));

        let dirty = determine_dirty_pages(
            &cache,
            &FxHashSet::default(),
            &FxHashSet::default(),
            &changed_assets,
        );

        assert!(dirty.contains(&key));
    }

    #[test]
    fn test_find_stale_pages() {
        let mut cached_pages = FxHashMap::default();
        let key_kept = PageKey::new_static("/", None);
        let key_removed = PageKey::new_static("/old-page", None);

        cached_pages.insert(
            key_kept.clone(),
            page_entry(vec![], vec![], "dist/index.html"),
        );
        cached_pages.insert(
            key_removed.clone(),
            page_entry(vec![], vec![], "dist/old-page/index.html"),
        );

        let mut current_pages = FxHashSet::default();
        current_pages.insert(key_kept.clone());

        let stale = find_stale_pages(&cached_pages, &current_pages);

        assert!(!stale.contains(&key_kept));
        assert!(stale.contains(&key_removed));
    }

    #[test]
    fn test_cache_roundtrip_with_pages() {
        let mut cache = BuildCache {
            version: BUILD_CACHE_VERSION,
            binary_hash: "test_hash".to_string(),
            ..Default::default()
        };

        let key = PageKey::new(
            "/articles/[slug]",
            &{
                let mut m = FxHashMap::default();
                m.insert("slug".to_string(), Some("hello".to_string()));
                m
            },
            Some("en"),
        );

        cache.pages.insert(
            key.clone(),
            PageCacheEntry {
                images: vec![CachedImage {
                    path: PathBuf::from("images/hero.jpg"),
                    hash: "img123".to_string(),
                }],
                scripts: vec![CachedScript {
                    path: PathBuf::from("script.js"),
                    hash: "js456".to_string(),
                    included: true,
                }],
                styles: vec![CachedStyle {
                    path: PathBuf::from("style.css"),
                    hash: "css789".to_string(),
                    included: true,
                    tailwind: false,
                }],
                ..page_entry(
                    vec![("articles".to_string(), "hello".to_string())],
                    vec![],
                    "dist/en/articles/hello/index.html",
                )
            },
        );

        let bytes = bincode::serialize(&cache).unwrap();
        let loaded: BuildCache = bincode::deserialize(&bytes).unwrap();

        assert_eq!(loaded.version, BUILD_CACHE_VERSION);
        assert_eq!(loaded.binary_hash, "test_hash");
        assert!(loaded.pages.contains_key(&key));

        let entry = &loaded.pages[&key];
        assert_eq!(entry.content_entries_read.len(), 1);
        assert_eq!(entry.images.len(), 1);
        assert_eq!(entry.scripts.len(), 1);
        assert_eq!(entry.styles.len(), 1);
        assert_eq!(
            entry.output_file,
            PathBuf::from("dist/en/articles/hello/index.html")
        );
    }

    #[test]
    fn test_page_key_sorted_params() {
        // PageKey params should be sorted, so insertion order doesn't matter
        let mut m1 = FxHashMap::default();
        m1.insert("a".to_string(), Some("1".to_string()));
        m1.insert("b".to_string(), Some("2".to_string()));

        let mut m2 = FxHashMap::default();
        m2.insert("b".to_string(), Some("2".to_string()));
        m2.insert("a".to_string(), Some("1".to_string()));

        let key1 = PageKey::new("/test/[a]/[b]", &m1, None);
        let key2 = PageKey::new("/test/[a]/[b]", &m2, None);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_structural_change_dirties_iterating_pages() {
        let mut pages = FxHashMap::default();
        let key_index = PageKey::new_static("/", None);
        pages.insert(
            key_index.clone(),
            page_entry(vec![], vec!["articles".to_string()], "dist/index.html"),
        );

        let key_about = PageKey::new_static("/about", None);
        pages.insert(
            key_about.clone(),
            page_entry(vec![], vec!["pages".to_string()], "dist/about/index.html"),
        );

        let cache = BuildCache {
            pages,
            ..Default::default()
        };

        // "articles" source changed structurally (entry added/removed)
        let mut structural = FxHashSet::default();
        structural.insert("articles".to_string());

        let dirty = determine_dirty_pages(
            &cache,
            &structural,
            &FxHashSet::default(),
            &FxHashSet::default(),
        );

        // Index iterates "articles" → dirty
        assert!(dirty.contains(&key_index));
        // About iterates "pages" (unchanged) → clean
        assert!(!dirty.contains(&key_about));
    }

    #[test]
    fn test_compute_content_source_state() {
        let empty_raw: FxHashMap<String, &str> = FxHashMap::default();

        // Test with empty entries
        let entries: Vec<(String, Vec<PathBuf>)> = vec![];
        let state = compute_content_source_state(&entries, &empty_raw);
        assert!(state.files.is_empty());
        assert!(state.entry_ids.is_empty());

        // Test with entries without file paths
        let entries = vec![("entry1".to_string(), vec![])];
        let state = compute_content_source_state(&entries, &empty_raw);
        assert!(state.files.is_empty());
        assert_eq!(state.entry_ids, vec!["entry1".to_string()]);

        // Test with raw content (should hash from memory, not read from disk)
        let mut raw = FxHashMap::default();
        raw.insert("entry1".to_string(), "hello world");
        let entries = vec![(
            "entry1".to_string(),
            vec![PathBuf::from("nonexistent/file.md")],
        )];
        let state = compute_content_source_state(&entries, &raw);
        let expected_hash = hash_bytes(b"hello world");
        assert_eq!(
            state.files.get(&PathBuf::from("nonexistent/file.md")),
            Some(&expected_hash)
        );
    }

    #[test]
    fn test_diff_new_source_is_structural_change() {
        let cached = FxHashMap::default();
        let mut current = FxHashMap::default();
        current.insert(
            "new_source".to_string(),
            ContentSourceState {
                files: FxHashMap::default(),
                entry_ids: vec!["a".to_string()],
                ..Default::default()
            },
        );

        let (structural, changed) = diff_content_sources(&cached, &current);
        assert!(structural.contains("new_source"));
        assert!(changed.is_empty());
    }

    #[test]
    fn test_diff_removed_source_is_structural_change() {
        let mut cached = FxHashMap::default();
        cached.insert(
            "old_source".to_string(),
            ContentSourceState {
                files: FxHashMap::default(),
                entry_ids: vec!["a".to_string()],
                ..Default::default()
            },
        );
        let current = FxHashMap::default();

        let (structural, _) = diff_content_sources(&cached, &current);
        assert!(structural.contains("old_source"));
    }

    #[test]
    fn test_iterated_source_content_change_dirties_page() {
        let mut pages = FxHashMap::default();
        let key = PageKey::new_static("/", None);
        pages.insert(
            key.clone(),
            page_entry(vec![], vec!["articles".to_string()], "dist/index.html"),
        );

        let cache = BuildCache {
            pages,
            ..Default::default()
        };

        // Content change in "articles" but no structural change
        let mut changed = FxHashSet::default();
        changed.insert(("articles".to_string(), "some-entry".to_string()));

        let dirty = determine_dirty_pages(
            &cache,
            &FxHashSet::default(),
            &changed,
            &FxHashSet::default(),
        );

        assert!(dirty.contains(&key));
    }

    #[test]
    fn test_no_changes_means_no_dirty_pages() {
        let mut pages = FxHashMap::default();
        let key = PageKey::new_static("/", None);
        pages.insert(
            key.clone(),
            PageCacheEntry {
                images: vec![CachedImage {
                    path: PathBuf::from("logo.png"),
                    hash: "abc".to_string(),
                }],
                ..page_entry(
                    vec![("articles".to_string(), "foo".to_string())],
                    vec![],
                    "dist/index.html",
                )
            },
        );

        let cache = BuildCache {
            pages,
            ..Default::default()
        };

        let dirty = determine_dirty_pages(
            &cache,
            &FxHashSet::default(),
            &FxHashSet::default(),
            &FxHashSet::default(),
        );

        assert!(dirty.is_empty());
    }

    #[test]
    fn test_cache_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path();

        let mut cache = BuildCache {
            version: BUILD_CACHE_VERSION,
            binary_hash: "test123".to_string(),
            ..Default::default()
        };

        let key = PageKey::new_static("/about", None);
        cache.pages.insert(
            key.clone(),
            page_entry(
                vec![("pages".to_string(), "about".to_string())],
                vec![],
                "dist/about/index.html",
            ),
        );

        cache.save(cache_dir).unwrap();

        let loaded = BuildCache::load(cache_dir).unwrap();
        assert_eq!(loaded.version, BUILD_CACHE_VERSION);
        assert_eq!(loaded.binary_hash, "test123");
        assert!(loaded.pages.contains_key(&key));
        assert_eq!(
            loaded.pages[&key].content_entries_read,
            vec![("pages".to_string(), "about".to_string())]
        );
    }

    #[test]
    fn test_cache_load_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path();
        let cache_file = cache_dir.join(BUILD_CACHE_FILENAME);

        fs::create_dir_all(cache_dir).unwrap();
        fs::write(&cache_file, b"this is not valid bincode").unwrap();

        assert!(BuildCache::load(cache_dir).is_none());
    }

    #[test]
    fn test_cache_load_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path();

        let cache = BuildCache {
            version: 999, // wrong version
            ..Default::default()
        };

        let bytes = bincode::serialize(&cache).unwrap();
        fs::create_dir_all(cache_dir).unwrap();
        fs::write(cache_dir.join(BUILD_CACHE_FILENAME), bytes).unwrap();

        assert!(BuildCache::load(cache_dir).is_none());
    }

    #[test]
    fn test_cache_load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(BuildCache::load(dir.path()).is_none());
    }

    #[test]
    fn test_diff_asset_files_detects_changes() {
        let dir = tempfile::tempdir().unwrap();
        let file_a = dir.path().join("style.css");
        let file_b = dir.path().join("script.js");

        fs::write(&file_a, "body { color: red; }").unwrap();
        fs::write(&file_b, "console.log('hi');").unwrap();

        // Build cached fingerprints
        let mut cached = FxHashMap::default();
        cached.insert(
            file_a.clone(),
            AssetFileFingerprint::from_path(&file_a).unwrap(),
        );
        cached.insert(
            file_b.clone(),
            AssetFileFingerprint::from_path(&file_b).unwrap(),
        );

        // No changes → empty diff
        assert!(diff_asset_files(&cached).is_empty());

        // Modify one file
        fs::write(&file_a, "body { color: blue; }").unwrap();
        let changed = diff_asset_files(&cached);
        assert!(changed.contains(&file_a));
        assert!(!changed.contains(&file_b));
    }

    #[test]
    fn test_diff_asset_files_deleted_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("gone.css");
        fs::write(&file, "body {}").unwrap();

        let mut cached = FxHashMap::default();
        cached.insert(
            file.clone(),
            AssetFileFingerprint::from_path(&file).unwrap(),
        );

        fs::remove_file(&file).unwrap();
        let changed = diff_asset_files(&cached);
        assert!(changed.contains(&file));
    }
}
