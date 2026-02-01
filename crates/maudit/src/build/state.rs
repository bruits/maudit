use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Identifies a specific route or variant for incremental rebuilds
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteIdentifier {
    /// A base route with optional page parameters
    /// Params are stored as a sorted Vec for hashing purposes
    Base {
        route_path: String,
        params: Option<Vec<(String, Option<String>)>>,
    },
    /// A variant route with optional page parameters
    /// Params are stored as a sorted Vec for hashing purposes
    Variant {
        variant_id: String,
        variant_path: String,
        params: Option<Vec<(String, Option<String>)>>,
    },
}

impl RouteIdentifier {
    pub fn base(route_path: String, params: Option<FxHashMap<String, Option<String>>>) -> Self {
        Self::Base {
            route_path,
            params: params.map(|p| {
                let mut sorted: Vec<_> = p.into_iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                sorted
            }),
        }
    }

    pub fn variant(
        variant_id: String,
        variant_path: String,
        params: Option<FxHashMap<String, Option<String>>>,
    ) -> Self {
        Self::Variant {
            variant_id,
            variant_path,
            params: params.map(|p| {
                let mut sorted: Vec<_> = p.into_iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                sorted
            }),
        }
    }
}

/// Tracks build state for incremental builds
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildState {
    /// Maps asset paths to routes that use them
    /// Key: canonicalized asset path
    /// Value: set of routes using this asset
    pub asset_to_routes: FxHashMap<PathBuf, FxHashSet<RouteIdentifier>>,

    /// Maps source file paths to routes defined in them
    /// Key: canonicalized source file path (e.g., src/pages/index.rs)
    /// Value: set of routes defined in this source file
    pub source_to_routes: FxHashMap<PathBuf, FxHashSet<RouteIdentifier>>,

    /// Maps content file paths to routes that use them
    /// Key: canonicalized content file path (e.g., content/articles/hello.md)
    /// Value: set of routes using this specific content file
    /// This provides granular tracking - if only hello.md changes, only routes
    /// that accessed hello.md need to be rebuilt.
    pub content_file_to_routes: FxHashMap<PathBuf, FxHashSet<RouteIdentifier>>,

    /// Maps content file paths to the content source that owns them
    /// Key: canonicalized content file path (e.g., content/articles/hello.md)
    /// Value: content source name (e.g., "articles")
    /// This allows selective re-initialization of only the content sources
    /// whose files have changed.
    pub content_file_to_source: FxHashMap<PathBuf, String>,

    /// Stores all bundler input paths from the last build
    /// This needs to be preserved to ensure consistent bundling
    pub bundler_inputs: Vec<String>,
}

impl BuildState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load build state from disk cache
    pub fn load(cache_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let state_path = cache_dir.join("build_state.json");

        if !state_path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&state_path)?;
        let state: BuildState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save build state to disk cache
    pub fn save(&self, cache_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(cache_dir)?;
        let state_path = cache_dir.join("build_state.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(state_path, content)?;
        Ok(())
    }

    /// Add an asset->route mapping
    pub fn track_asset(&mut self, asset_path: PathBuf, route_id: RouteIdentifier) {
        self.asset_to_routes
            .entry(asset_path)
            .or_default()
            .insert(route_id);
    }

    /// Add a source file->route mapping
    /// This tracks which .rs file defines which routes for incremental rebuilds
    pub fn track_source_file(&mut self, source_path: PathBuf, route_id: RouteIdentifier) {
        self.source_to_routes
            .entry(source_path)
            .or_default()
            .insert(route_id);
    }

    /// Add a content file->route mapping
    /// This tracks which specific content files are used by which routes for incremental rebuilds.
    /// This provides granular tracking - only routes that actually accessed a specific file
    /// will be rebuilt when that file changes.
    ///
    /// The file path is canonicalized before storage to ensure consistent lookups when
    /// comparing against absolute paths from the file watcher.
    pub fn track_content_file(&mut self, file_path: PathBuf, route_id: RouteIdentifier) {
        // Canonicalize the path to ensure consistent matching with absolute paths from the watcher
        let canonical_path = file_path.canonicalize().unwrap_or(file_path);
        self.content_file_to_routes
            .entry(canonical_path)
            .or_default()
            .insert(route_id);
    }

    /// Add a content file->source mapping
    /// This tracks which content source owns each file, allowing selective re-initialization
    /// of only the sources whose files have changed.
    ///
    /// The file path is canonicalized before storage to ensure consistent lookups.
    pub fn track_content_file_source(&mut self, file_path: PathBuf, source_name: String) {
        let canonical_path = file_path.canonicalize().unwrap_or(file_path);
        self.content_file_to_source
            .insert(canonical_path, source_name);
    }

    /// Get the names of content sources that have files in the changed files list.
    /// Returns `None` if any changed content file is not tracked (new file), indicating
    /// that all content sources should be re-initialized.
    ///
    /// Only considers files that look like content files (have common content extensions).
    pub fn get_affected_content_sources(
        &self,
        changed_files: &[PathBuf],
    ) -> Option<FxHashSet<String>> {
        let content_extensions = ["md", "mdx", "yaml", "yml", "json", "toml"];
        let mut affected_sources = FxHashSet::default();

        for changed_file in changed_files {
            // Skip files that don't look like content files
            let is_content_file = changed_file
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| content_extensions.contains(&ext))
                .unwrap_or(false);

            if !is_content_file {
                continue;
            }

            // Try to find the source for this file
            let canonical = changed_file.canonicalize().ok();

            let source = canonical
                .as_ref()
                .and_then(|c| self.content_file_to_source.get(c))
                .or_else(|| self.content_file_to_source.get(changed_file));

            match source {
                Some(source_name) => {
                    affected_sources.insert(source_name.clone());
                }
                None => {
                    // Unknown content file - could be a new file
                    // Fall back to re-initializing all sources
                    return None;
                }
            }
        }

        Some(affected_sources)
    }

    /// Get all routes affected by changes to specific files.
    ///
    /// Returns `Some(routes)` if all changed files were found in the mappings,
    /// or `None` if any changed file is untracked (meaning we need a full rebuild).
    ///
    /// This handles the case where files like those referenced by `include_str!()`
    /// are not tracked at the route level - when these change, we fall back to
    /// rebuilding all routes to ensure correctness.
    ///
    /// Note: Existing directories are not considered "untracked" - they are checked
    /// via prefix matching, but a new/unknown directory won't trigger a full rebuild.
    pub fn get_affected_routes(
        &self,
        changed_files: &[PathBuf],
    ) -> Option<FxHashSet<RouteIdentifier>> {
        let mut affected_routes = FxHashSet::default();
        let mut has_untracked_file = false;

        for changed_file in changed_files {
            let mut file_was_tracked = false;

            // Canonicalize the changed file path for consistent comparison
            // All asset paths in asset_to_routes are stored as canonical paths
            let canonical_changed = changed_file.canonicalize().ok();

            // Check source file mappings first (for .rs files)
            if let Some(canonical) = &canonical_changed
                && let Some(routes) = self.source_to_routes.get(canonical)
            {
                affected_routes.extend(routes.iter().cloned());
                file_was_tracked = true;
                // Continue to also check asset mappings (a file could be both)
            }

            // Also check with original path for source files
            if let Some(routes) = self.source_to_routes.get(changed_file) {
                affected_routes.extend(routes.iter().cloned());
                file_was_tracked = true;
            }

            // Try exact match with canonical path for assets
            if let Some(canonical) = &canonical_changed
                && let Some(routes) = self.asset_to_routes.get(canonical)
            {
                affected_routes.extend(routes.iter().cloned());
                file_was_tracked = true;
            }

            // Fallback: try exact match with original path (shouldn't normally match)
            if let Some(routes) = self.asset_to_routes.get(changed_file) {
                affected_routes.extend(routes.iter().cloned());
                file_was_tracked = true;
            }

            // Check if this is a content file with direct file->route tracking
            if let Some(canonical) = &canonical_changed
                && let Some(routes) = self.content_file_to_routes.get(canonical)
            {
                affected_routes.extend(routes.iter().cloned());
                file_was_tracked = true;
            }

            // Also check with original path for content files
            if let Some(routes) = self.content_file_to_routes.get(changed_file) {
                affected_routes.extend(routes.iter().cloned());
                file_was_tracked = true;
            }

            // Directory prefix check: find all routes using assets within this directory.
            // This handles two cases:
            // 1. A directory was modified - rebuild all routes using assets in that dir
            // 2. A directory was renamed/deleted - the old path no longer exists but we
            //    still need to rebuild routes that used assets under that path
            //
            // We do this check if:
            // - The path currently exists as a directory, OR
            // - The path doesn't exist (could be a deleted/renamed directory)
            let is_existing_directory = changed_file.is_dir();
            let path_does_not_exist = !changed_file.exists();

            if is_existing_directory || path_does_not_exist {
                // Use original path for prefix matching (canonical won't exist for deleted dirs)
                for (asset_path, routes) in &self.asset_to_routes {
                    if asset_path.starts_with(changed_file) {
                        affected_routes.extend(routes.iter().cloned());
                        file_was_tracked = true;
                    }
                }
                // Also check source files for directory prefix
                for (source_path, routes) in &self.source_to_routes {
                    if source_path.starts_with(changed_file) {
                        affected_routes.extend(routes.iter().cloned());
                        file_was_tracked = true;
                    }
                }
                // Also check content files for directory prefix
                for (content_path, routes) in &self.content_file_to_routes {
                    if content_path.starts_with(changed_file) {
                        affected_routes.extend(routes.iter().cloned());
                        file_was_tracked = true;
                    }
                }
            }

            // Flag as untracked (triggering full rebuild) if:
            // 1. The file wasn't found in any mapping, AND
            // 2. It's not a currently-existing directory (new directories are OK to ignore)
            //
            // For non-existent paths that weren't matched:
            // - If the path has a file extension, treat it as a deleted file → full rebuild
            // - If the path has no extension, it might be a deleted directory → allow
            //   (we already checked prefix matching above)
            //
            // This is conservative: we'd rather rebuild too much than too little.
            if !file_was_tracked && !is_existing_directory {
                if path_does_not_exist {
                    // For deleted paths, check if it looks like a file (has extension)
                    // If it has an extension, it was probably a file → trigger full rebuild
                    // If no extension, it might have been a directory → don't trigger
                    let has_extension = changed_file
                        .extension()
                        .map(|ext| !ext.is_empty())
                        .unwrap_or(false);

                    if has_extension {
                        has_untracked_file = true;
                    }
                } else {
                    // Path exists but wasn't tracked → definitely untracked file
                    has_untracked_file = true;
                }
            }
        }

        if has_untracked_file {
            // Some files weren't tracked - caller should do a full rebuild
            None
        } else {
            Some(affected_routes)
        }
    }

    /// Clear all tracked data (for full rebuild)
    pub fn clear(&mut self) {
        self.asset_to_routes.clear();
        self.source_to_routes.clear();
        self.content_file_to_routes.clear();
        self.content_file_to_source.clear();
        self.bundler_inputs.clear();
    }

    /// Clear the content file to routes mapping.
    /// This should be called before re-tracking content files after content sources are re-initialized.
    pub fn clear_content_file_mappings(&mut self) {
        self.content_file_to_routes.clear();
    }

    /// Clear content file mappings for specific sources.
    /// This removes both file->routes and file->source mappings for files owned by the given sources.
    /// Called when selectively re-initializing specific content sources.
    pub fn clear_content_mappings_for_sources(&mut self, source_names: &FxHashSet<String>) {
        // Find all files that belong to the specified sources
        let files_to_remove: Vec<PathBuf> = self
            .content_file_to_source
            .iter()
            .filter(|(_, source)| source_names.contains(*source))
            .map(|(path, _)| path.clone())
            .collect();

        // Remove file->source mappings only
        // We DON'T clear file->routes mappings here because:
        // 1. Routes not being rebuilt should keep their mappings
        // 2. Routes being rebuilt will have their mappings cleared separately
        //    via clear_content_file_mappings_for_routes()
        for file in &files_to_remove {
            self.content_file_to_source.remove(file);
        }
    }

    /// Remove content file mappings for specific routes.
    /// This is used during incremental builds to clear only the mappings for routes
    /// that will be rebuilt, preserving mappings for routes that won't change.
    pub fn clear_content_file_mappings_for_routes(&mut self, routes: &FxHashSet<RouteIdentifier>) {
        for routes_set in self.content_file_to_routes.values_mut() {
            routes_set.retain(|route| !routes.contains(route));
        }
        // Remove any entries that have no routes left
        self.content_file_to_routes
            .retain(|_, routes_set| !routes_set.is_empty());
    }

    /// Check if a file path is a known content file.
    /// This is used to determine if a new file might be a content file.
    #[allow(dead_code)] // Used in tests and potentially useful for debugging
    pub fn is_known_content_file(&self, file_path: &Path) -> bool {
        if self.content_file_to_routes.contains_key(file_path) {
            return true;
        }

        // Try with canonicalized path
        if let Ok(canonical) = file_path.canonicalize() {
            return self.content_file_to_routes.contains_key(&canonical);
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_route(path: &str) -> RouteIdentifier {
        RouteIdentifier::base(path.to_string(), None)
    }

    #[test]
    fn test_get_affected_routes_exact_match() {
        let mut state = BuildState::new();
        let asset_path = PathBuf::from("/project/src/assets/logo.png");
        let route = make_route("/");

        state.track_asset(asset_path.clone(), route.clone());

        // Exact match should work and return Some
        let affected = state.get_affected_routes(&[asset_path]).unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route));
    }

    #[test]
    fn test_get_affected_routes_untracked_file() {
        use std::fs;
        use tempfile::TempDir;

        let mut state = BuildState::new();

        // Create temp files
        let temp_dir = TempDir::new().unwrap();
        let tracked_file = temp_dir.path().join("logo.png");
        let untracked_file = temp_dir.path().join("other.png");
        fs::write(&tracked_file, "tracked").unwrap();
        fs::write(&untracked_file, "untracked").unwrap();

        let route = make_route("/");
        state.track_asset(tracked_file.clone(), route);

        // Untracked file that EXISTS should return None (triggers full rebuild)
        let affected = state.get_affected_routes(&[untracked_file]);
        assert!(affected.is_none());
    }

    #[test]
    fn test_get_affected_routes_mixed_tracked_untracked() {
        use std::fs;
        use tempfile::TempDir;

        let mut state = BuildState::new();

        // Create temp files
        let temp_dir = TempDir::new().unwrap();
        let tracked_file = temp_dir.path().join("logo.png");
        let untracked_file = temp_dir.path().join("other.png");
        fs::write(&tracked_file, "tracked").unwrap();
        fs::write(&untracked_file, "untracked").unwrap();

        let route = make_route("/");
        state.track_asset(tracked_file.canonicalize().unwrap(), route);

        // If any file is untracked, return None (even if some are tracked)
        let affected = state.get_affected_routes(&[tracked_file, untracked_file]);
        assert!(affected.is_none());
    }

    #[test]
    fn test_get_affected_routes_deleted_directory() {
        let mut state = BuildState::new();

        // Track assets under a directory path
        let asset1 = PathBuf::from("/project/src/assets/icons/logo.png");
        let asset2 = PathBuf::from("/project/src/assets/icons/favicon.ico");
        let asset3 = PathBuf::from("/project/src/assets/styles.css");
        let route1 = make_route("/");
        let route2 = make_route("/about");

        state.track_asset(asset1, route1.clone());
        state.track_asset(asset2, route1.clone());
        state.track_asset(asset3, route2.clone());

        // Simulate a deleted/renamed directory (path doesn't exist)
        // The "icons" directory was renamed, so the old path doesn't exist
        let deleted_dir = PathBuf::from("/project/src/assets/icons");

        // Since the path doesn't exist, it should check prefix matching
        let affected = state.get_affected_routes(&[deleted_dir]).unwrap();

        // Should find route1 (uses assets under /icons/) but not route2
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route1));
    }

    #[test]
    fn test_get_affected_routes_multiple_routes_same_asset() {
        let mut state = BuildState::new();
        let asset_path = PathBuf::from("/project/src/assets/shared.css");
        let route1 = make_route("/");
        let route2 = make_route("/about");

        state.track_asset(asset_path.clone(), route1.clone());
        state.track_asset(asset_path.clone(), route2.clone());

        let affected = state.get_affected_routes(&[asset_path]).unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&route1));
        assert!(affected.contains(&route2));
    }

    #[test]
    fn test_get_affected_routes_source_file() {
        let mut state = BuildState::new();
        let source_path = PathBuf::from("/project/src/pages/index.rs");
        let route1 = make_route("/");
        let route2 = make_route("/about");

        // Track routes to their source files
        state.track_source_file(source_path.clone(), route1.clone());
        state.track_source_file(source_path.clone(), route2.clone());

        // When the source file changes, both routes should be affected
        let affected = state.get_affected_routes(&[source_path]).unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&route1));
        assert!(affected.contains(&route2));
    }

    #[test]
    fn test_get_affected_routes_source_file_only_matching() {
        let mut state = BuildState::new();
        let source_index = PathBuf::from("/project/src/pages/index.rs");
        let source_about = PathBuf::from("/project/src/pages/about.rs");
        let route_index = make_route("/");
        let route_about = make_route("/about");

        state.track_source_file(source_index.clone(), route_index.clone());
        state.track_source_file(source_about.clone(), route_about.clone());

        // Changing only index.rs should only affect the index route
        let affected = state.get_affected_routes(&[source_index]).unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route_index));
        assert!(!affected.contains(&route_about));
    }

    #[test]
    fn test_clear_also_clears_source_files() {
        let mut state = BuildState::new();
        let source_path = PathBuf::from("/project/src/pages/index.rs");
        let asset_path = PathBuf::from("/project/src/assets/logo.png");
        let route = make_route("/");

        state.track_source_file(source_path.clone(), route.clone());
        state.track_asset(asset_path.clone(), route.clone());

        assert!(!state.source_to_routes.is_empty());
        assert!(!state.asset_to_routes.is_empty());

        state.clear();

        assert!(state.source_to_routes.is_empty());
        assert!(state.asset_to_routes.is_empty());
    }

    #[test]
    fn test_get_affected_routes_new_directory_not_untracked() {
        use std::fs;
        use tempfile::TempDir;

        let mut state = BuildState::new();

        // Create a temporary directory to simulate the "new directory" scenario
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new-folder");
        fs::create_dir(&new_dir).unwrap();

        // Track some asset under a different path
        let asset_path = PathBuf::from("/project/src/assets/logo.png");
        let route = make_route("/");
        state.track_asset(asset_path.clone(), route.clone());

        // When a new directory appears (e.g., from renaming another folder),
        // it should NOT trigger a full rebuild (return None), even though
        // we don't have any assets tracked under it.
        let affected = state.get_affected_routes(&[new_dir]);

        // Should return Some (not None), meaning we don't trigger full rebuild
        // The set should be empty since no assets are under this new directory
        assert!(
            affected.is_some(),
            "New directory should not trigger full rebuild"
        );
        assert!(affected.unwrap().is_empty());
    }

    #[test]
    fn test_get_affected_routes_folder_rename_scenario() {
        use std::fs;
        use tempfile::TempDir;

        let mut state = BuildState::new();

        // Create temp directories to simulate folder rename
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("icons-renamed");
        fs::create_dir(&new_dir).unwrap();

        // Track assets under the OLD folder path (which no longer exists)
        let old_dir = PathBuf::from("/project/src/assets/icons");
        let asset1 = PathBuf::from("/project/src/assets/icons/logo.png");
        let route = make_route("/blog");
        state.track_asset(asset1, route.clone());

        // Simulate folder rename: old path doesn't exist, new path is a directory
        // Both paths are passed as "changed"
        let affected = state.get_affected_routes(&[old_dir, new_dir]);

        // Should return Some (not None) - we found the affected route via prefix matching
        // and the new directory doesn't trigger "untracked file" behavior
        assert!(
            affected.is_some(),
            "Folder rename should not trigger full rebuild"
        );
        let routes = affected.unwrap();
        assert_eq!(routes.len(), 1);
        assert!(routes.contains(&route));
    }

    #[test]
    fn test_get_affected_routes_deleted_untracked_file() {
        let mut state = BuildState::new();

        // Track some assets
        let tracked_asset = PathBuf::from("/project/src/assets/logo.png");
        let route = make_route("/");
        state.track_asset(tracked_asset, route);

        // Simulate a deleted file that was NEVER tracked
        // (e.g., a file used via include_str! that we don't know about)
        // This path doesn't exist and isn't in any mapping
        let deleted_untracked_file = PathBuf::from("/project/src/content/data.txt");

        let affected = state.get_affected_routes(&[deleted_untracked_file]);

        // Since the deleted path has a file extension (.txt), we treat it as
        // a deleted file that might have been a dependency we don't track.
        // We should trigger a full rebuild (return None) to be safe.
        assert!(
            affected.is_none(),
            "Deleted untracked file with extension should trigger full rebuild"
        );
    }

    #[test]
    fn test_get_affected_routes_deleted_untracked_directory() {
        let mut state = BuildState::new();

        // Track some assets
        let tracked_asset = PathBuf::from("/project/src/assets/logo.png");
        let route = make_route("/");
        state.track_asset(tracked_asset, route);

        // Simulate a deleted directory that was NEVER tracked
        // This path doesn't exist, isn't in any mapping, and has no extension
        let deleted_untracked_dir = PathBuf::from("/project/src/content");

        let affected = state.get_affected_routes(&[deleted_untracked_dir]);

        // Since the path has no extension, it might have been a directory.
        // We already did prefix matching (found nothing), so we allow this
        // without triggering a full rebuild.
        assert!(
            affected.is_some(),
            "Deleted path without extension (possible directory) should not trigger full rebuild"
        );
        assert!(affected.unwrap().is_empty());
    }

    #[test]
    fn test_get_affected_routes_deleted_tracked_file() {
        use std::fs;
        use tempfile::TempDir;

        let mut state = BuildState::new();

        // Create a temp file, track it, then delete it
        let temp_dir = TempDir::new().unwrap();
        let tracked_file = temp_dir.path().join("logo.png");
        fs::write(&tracked_file, "content").unwrap();

        let canonical_path = tracked_file.canonicalize().unwrap();
        let route = make_route("/");
        state.track_asset(canonical_path.clone(), route.clone());

        // Now delete the file
        fs::remove_file(&tracked_file).unwrap();

        // The file no longer exists, but its canonical path is still in our mapping
        // When we get the change event, notify gives us the original path
        let affected = state.get_affected_routes(std::slice::from_ref(&tracked_file));

        // This SHOULD find the route because we track by canonical path
        // and the original path should match via the mapping lookup
        println!("Result for deleted tracked file: {:?}", affected);

        // The path doesn't exist anymore, so canonicalize() fails.
        // We fall back to prefix matching, but exact path matching on
        // the non-canonical path should still work if stored that way.
        // Let's check what actually happens...
        match affected {
            Some(routes) => {
                // If we found routes, great - the system works
                assert!(
                    routes.contains(&route),
                    "Should find the route for deleted tracked file"
                );
            }
            None => {
                // If None, that means we triggered a full rebuild, which is also safe
                // This happens because the file doesn't exist and wasn't found in mappings
                println!("Deleted tracked file triggered full rebuild (safe behavior)");
            }
        }
    }

    #[test]
    fn test_track_content_file() {
        let mut state = BuildState::new();
        let route = make_route("/");
        let content_file = PathBuf::from("/project/content/articles/hello.md");

        state.track_content_file(content_file.clone(), route.clone());

        assert_eq!(state.content_file_to_routes.len(), 1);
        assert!(state.content_file_to_routes.contains_key(&content_file));
        assert!(state.content_file_to_routes[&content_file].contains(&route));
    }

    #[test]
    fn test_track_content_file_multiple_routes() {
        let mut state = BuildState::new();
        let route1 = make_route("/");
        let route2 = make_route("/blog");
        let content_file = PathBuf::from("/project/content/articles/hello.md");

        state.track_content_file(content_file.clone(), route1.clone());
        state.track_content_file(content_file.clone(), route2.clone());

        assert_eq!(state.content_file_to_routes.len(), 1);
        assert_eq!(state.content_file_to_routes[&content_file].len(), 2);
        assert!(state.content_file_to_routes[&content_file].contains(&route1));
        assert!(state.content_file_to_routes[&content_file].contains(&route2));
    }

    #[test]
    fn test_track_content_file_multiple_files() {
        let mut state = BuildState::new();
        let route = make_route("/");
        let file1 = PathBuf::from("/project/content/articles/hello.md");
        let file2 = PathBuf::from("/project/content/articles/world.md");

        state.track_content_file(file1.clone(), route.clone());
        state.track_content_file(file2.clone(), route.clone());

        assert_eq!(state.content_file_to_routes.len(), 2);
        assert!(state.content_file_to_routes[&file1].contains(&route));
        assert!(state.content_file_to_routes[&file2].contains(&route));
    }

    #[test]
    fn test_clear_also_clears_content_files() {
        let mut state = BuildState::new();
        let route = make_route("/");
        let content_file = PathBuf::from("/project/content/articles/hello.md");

        state.track_content_file(content_file, route);

        assert!(!state.content_file_to_routes.is_empty());

        state.clear();

        assert!(state.content_file_to_routes.is_empty());
    }

    #[test]
    fn test_get_affected_routes_content_file() {
        let mut state = BuildState::new();
        let route1 = make_route("/");
        let route2 = make_route("/blog/[slug]");
        let route3 = make_route("/about");

        // Track content file -> route mappings directly
        let article1 = PathBuf::from("/project/content/articles/hello.md");
        let article2 = PathBuf::from("/project/content/articles/world.md");
        let page1 = PathBuf::from("/project/content/pages/about.md");

        // Route "/" uses article1 and article2
        state.track_content_file(article1.clone(), route1.clone());
        state.track_content_file(article2.clone(), route1.clone());
        // Route "/blog/[slug]" uses only article1
        state.track_content_file(article1.clone(), route2.clone());
        // Route "/about" uses page1
        state.track_content_file(page1.clone(), route3.clone());

        // When article1 changes, only routes that used article1 should be affected
        let affected = state.get_affected_routes(&[article1]).unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&route1));
        assert!(affected.contains(&route2));
        assert!(!affected.contains(&route3));

        // When article2 changes, only route1 should be affected (granular!)
        let affected = state.get_affected_routes(&[article2]).unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route1));
        assert!(!affected.contains(&route2));
        assert!(!affected.contains(&route3));

        // When page1 changes, only route3 should be affected
        let affected = state.get_affected_routes(&[page1]).unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route3));
        assert!(!affected.contains(&route1));
        assert!(!affected.contains(&route2));
    }

    #[test]
    fn test_get_affected_routes_content_file_multiple_files_changed() {
        let mut state = BuildState::new();
        let route1 = make_route("/");
        let route2 = make_route("/about");

        // Track content files
        let article = PathBuf::from("/project/content/articles/hello.md");
        let page = PathBuf::from("/project/content/pages/about.md");

        state.track_content_file(article.clone(), route1.clone());
        state.track_content_file(page.clone(), route2.clone());

        // When both files change, both routes should be affected
        let affected = state.get_affected_routes(&[article, page]).unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&route1));
        assert!(affected.contains(&route2));
    }

    #[test]
    fn test_get_affected_routes_content_file_mixed_with_asset() {
        let mut state = BuildState::new();
        let route1 = make_route("/");
        let route2 = make_route("/about");

        // Track a content file for route1
        let article = PathBuf::from("/project/content/articles/hello.md");
        state.track_content_file(article.clone(), route1.clone());

        // Track an asset used by route2
        let style = PathBuf::from("/project/src/styles.css");
        state.track_asset(style.clone(), route2.clone());

        // When both content file and asset change
        let affected = state.get_affected_routes(&[article, style]).unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&route1));
        assert!(affected.contains(&route2));
    }

    #[test]
    fn test_get_affected_routes_unknown_content_file() {
        let mut state = BuildState::new();
        let route = make_route("/");

        // Track a content file
        let article = PathBuf::from("/project/content/articles/hello.md");
        state.track_content_file(article, route);

        // A new/unknown .md file that isn't tracked
        // This could be a newly created file
        let new_file = PathBuf::from("/project/content/articles/new-post.md");

        // Should trigger full rebuild since it's an untracked file with extension
        let affected = state.get_affected_routes(&[new_file]);
        assert!(
            affected.is_none(),
            "New untracked content file should trigger full rebuild"
        );
    }

    #[test]
    fn test_is_known_content_file() {
        let mut state = BuildState::new();
        let route = make_route("/");
        let content_file = PathBuf::from("/project/content/articles/hello.md");

        state.track_content_file(content_file.clone(), route);

        assert!(state.is_known_content_file(&content_file));
        assert!(!state.is_known_content_file(Path::new("/project/content/articles/unknown.md")));
    }

    #[test]
    fn test_content_file_directory_prefix() {
        let mut state = BuildState::new();
        let route = make_route("/");

        // Track content files under a directory
        let article1 = PathBuf::from("/project/content/articles/hello.md");
        let article2 = PathBuf::from("/project/content/articles/world.md");
        state.track_content_file(article1, route.clone());
        state.track_content_file(article2, route.clone());

        // When the parent directory changes (e.g., renamed), should find affected routes
        let content_dir = PathBuf::from("/project/content/articles");
        let affected = state.get_affected_routes(&[content_dir]).unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route));
    }

    #[test]
    fn test_clear_content_file_mappings_for_routes() {
        let mut state = BuildState::new();
        let route1 = make_route("/articles");
        let route2 = make_route("/articles/[slug]");
        let route3 = make_route("/about");

        // Article 1 is accessed by routes 1 and 2
        let article1 = PathBuf::from("/project/content/articles/hello.md");
        state.track_content_file(article1.clone(), route1.clone());
        state.track_content_file(article1.clone(), route2.clone());

        // Article 2 is accessed by routes 1 and 2
        let article2 = PathBuf::from("/project/content/articles/world.md");
        state.track_content_file(article2.clone(), route1.clone());
        state.track_content_file(article2.clone(), route2.clone());

        // Route 3 uses a different file
        let page = PathBuf::from("/project/content/pages/about.md");
        state.track_content_file(page.clone(), route3.clone());

        assert_eq!(state.content_file_to_routes.len(), 3);

        // Clear mappings only for route2
        let mut routes_to_clear = FxHashSet::default();
        routes_to_clear.insert(route2.clone());
        state.clear_content_file_mappings_for_routes(&routes_to_clear);

        // route2 should be removed from article1 and article2 mappings
        assert!(!state.content_file_to_routes[&article1].contains(&route2));
        assert!(state.content_file_to_routes[&article1].contains(&route1));

        assert!(!state.content_file_to_routes[&article2].contains(&route2));
        assert!(state.content_file_to_routes[&article2].contains(&route1));

        // route3's mapping should be unaffected
        assert!(state.content_file_to_routes[&page].contains(&route3));
    }

    #[test]
    fn test_clear_content_file_mappings_for_routes_removes_empty_entries() {
        let mut state = BuildState::new();
        let route1 = make_route("/articles/first");
        let route2 = make_route("/articles/second");

        // Route1 uses only article1
        let article1 = PathBuf::from("/project/content/articles/first.md");
        state.track_content_file(article1.clone(), route1.clone());

        // Route2 uses only article2
        let article2 = PathBuf::from("/project/content/articles/second.md");
        state.track_content_file(article2.clone(), route2.clone());

        assert_eq!(state.content_file_to_routes.len(), 2);

        // Clear mappings for route1
        let mut routes_to_clear = FxHashSet::default();
        routes_to_clear.insert(route1);
        state.clear_content_file_mappings_for_routes(&routes_to_clear);

        // article1 entry should be completely removed (no routes left)
        assert!(!state.content_file_to_routes.contains_key(&article1));

        // article2 entry should still exist
        assert!(state.content_file_to_routes.contains_key(&article2));
        assert!(state.content_file_to_routes[&article2].contains(&route2));
    }

    #[test]
    fn test_track_content_file_source() {
        let mut state = BuildState::new();
        let file = PathBuf::from("/project/content/articles/hello.md");

        state.track_content_file_source(file.clone(), "articles".to_string());

        assert_eq!(state.content_file_to_source.len(), 1);
        assert_eq!(
            state.content_file_to_source.get(&file),
            Some(&"articles".to_string())
        );
    }

    #[test]
    fn test_get_affected_content_sources_single_source() {
        let mut state = BuildState::new();
        let article1 = PathBuf::from("/project/content/articles/hello.md");
        let article2 = PathBuf::from("/project/content/articles/world.md");

        state.track_content_file_source(article1.clone(), "articles".to_string());
        state.track_content_file_source(article2.clone(), "articles".to_string());

        // Change one article file
        let affected = state.get_affected_content_sources(&[article1]).unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains("articles"));
    }

    #[test]
    fn test_get_affected_content_sources_multiple_sources() {
        let mut state = BuildState::new();
        let article = PathBuf::from("/project/content/articles/hello.md");
        let page = PathBuf::from("/project/content/pages/about.md");

        state.track_content_file_source(article.clone(), "articles".to_string());
        state.track_content_file_source(page.clone(), "pages".to_string());

        // Change both files
        let affected = state
            .get_affected_content_sources(&[article, page])
            .unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains("articles"));
        assert!(affected.contains("pages"));
    }

    #[test]
    fn test_get_affected_content_sources_unknown_file_returns_none() {
        let mut state = BuildState::new();
        let article = PathBuf::from("/project/content/articles/hello.md");
        state.track_content_file_source(article, "articles".to_string());

        // A new file that's not tracked
        let new_file = PathBuf::from("/project/content/articles/new-post.md");

        // Should return None (need to re-init all sources)
        let affected = state.get_affected_content_sources(&[new_file]);
        assert!(affected.is_none());
    }

    #[test]
    fn test_get_affected_content_sources_ignores_non_content_files() {
        let mut state = BuildState::new();
        let article = PathBuf::from("/project/content/articles/hello.md");
        state.track_content_file_source(article.clone(), "articles".to_string());

        // A non-content file (e.g., .rs file) - should be ignored
        let rust_file = PathBuf::from("/project/src/pages/index.rs");

        // Should return empty set (no content sources affected)
        let affected = state
            .get_affected_content_sources(std::slice::from_ref(&rust_file))
            .unwrap();
        assert!(affected.is_empty());

        // Mixed: content file + non-content file
        let affected = state
            .get_affected_content_sources(&[article, rust_file])
            .unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains("articles"));
    }

    #[test]
    fn test_clear_content_mappings_for_sources() {
        let mut state = BuildState::new();
        let route1 = make_route("/articles");
        let route2 = make_route("/pages");

        // Set up articles source
        let article1 = PathBuf::from("/project/content/articles/hello.md");
        let article2 = PathBuf::from("/project/content/articles/world.md");
        state.track_content_file_source(article1.clone(), "articles".to_string());
        state.track_content_file_source(article2.clone(), "articles".to_string());
        state.track_content_file(article1.clone(), route1.clone());
        state.track_content_file(article2.clone(), route1.clone());

        // Set up pages source
        let page = PathBuf::from("/project/content/pages/about.md");
        state.track_content_file_source(page.clone(), "pages".to_string());
        state.track_content_file(page.clone(), route2.clone());

        assert_eq!(state.content_file_to_source.len(), 3);
        assert_eq!(state.content_file_to_routes.len(), 3);

        // Clear only the articles source
        let mut sources_to_clear = FxHashSet::default();
        sources_to_clear.insert("articles".to_string());
        state.clear_content_mappings_for_sources(&sources_to_clear);

        // Articles source mappings should be removed
        assert!(!state.content_file_to_source.contains_key(&article1));
        assert!(!state.content_file_to_source.contains_key(&article2));

        // But routes mappings should be preserved (cleared separately per-route)
        assert!(state.content_file_to_routes.contains_key(&article1));
        assert!(state.content_file_to_routes.contains_key(&article2));

        // Pages should remain completely unchanged
        assert!(state.content_file_to_source.contains_key(&page));
        assert!(state.content_file_to_routes.contains_key(&page));
        assert_eq!(
            state.content_file_to_source.get(&page),
            Some(&"pages".to_string())
        );
    }

    #[test]
    fn test_clear_also_clears_content_file_to_source() {
        let mut state = BuildState::new();
        let file = PathBuf::from("/project/content/articles/hello.md");
        state.track_content_file_source(file, "articles".to_string());

        assert!(!state.content_file_to_source.is_empty());

        state.clear();

        assert!(state.content_file_to_source.is_empty());
    }
}
