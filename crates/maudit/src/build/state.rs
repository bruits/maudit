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

    /// Get all routes affected by changes to specific files
    pub fn get_affected_routes(&self, changed_files: &[PathBuf]) -> FxHashSet<RouteIdentifier> {
        let mut affected_routes = FxHashSet::default();

        for changed_file in changed_files {
            // Canonicalize the changed file path for consistent comparison
            // All asset paths in asset_to_routes are stored as canonical paths
            let canonical_changed = changed_file.canonicalize().ok();

            // Try exact match with canonical path
            if let Some(canonical) = &canonical_changed
                && let Some(routes) = self.asset_to_routes.get(canonical)
            {
                affected_routes.extend(routes.iter().cloned());
                continue; // Found exact match, no need for directory check
            }

            // Fallback: try exact match with original path (shouldn't normally match)
            if let Some(routes) = self.asset_to_routes.get(changed_file) {
                affected_routes.extend(routes.iter().cloned());
                continue;
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
            let should_check_prefix = changed_file.is_dir() || !changed_file.exists();

            if should_check_prefix {
                // Use original path for prefix matching (canonical won't exist for deleted dirs)
                for (asset_path, routes) in &self.asset_to_routes {
                    if asset_path.starts_with(changed_file) {
                        affected_routes.extend(routes.iter().cloned());
                    }
                }
            }
        }

        affected_routes
    }

    /// Clear all tracked data (for full rebuild)
    pub fn clear(&mut self) {
        self.asset_to_routes.clear();
        self.bundler_inputs.clear();
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

        // Exact match should work
        let affected = state.get_affected_routes(&[asset_path]);
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&route));
    }

    #[test]
    fn test_get_affected_routes_no_match() {
        let mut state = BuildState::new();
        let asset_path = PathBuf::from("/project/src/assets/logo.png");
        let route = make_route("/");

        state.track_asset(asset_path, route);

        // Different file should not match
        let other_path = PathBuf::from("/project/src/assets/other.png");
        let affected = state.get_affected_routes(&[other_path]);
        assert!(affected.is_empty());
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
        let affected = state.get_affected_routes(&[deleted_dir]);

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

        let affected = state.get_affected_routes(&[asset_path]);
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&route1));
        assert!(affected.contains(&route2));
    }
}
