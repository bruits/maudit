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
            if let Some(canonical) = &canonical_changed {
                if let Some(routes) = self.asset_to_routes.get(canonical) {
                    affected_routes.extend(routes.iter().cloned());
                    continue; // Found exact match, no need for directory check
                }
            }

            // Fallback: try exact match with original path (shouldn't normally match)
            if let Some(routes) = self.asset_to_routes.get(changed_file) {
                affected_routes.extend(routes.iter().cloned());
                continue;
            }

            // Only do directory prefix check if the changed path is actually a directory
            // This handles cases where a directory is modified and we want to rebuild
            // all routes that use assets within that directory
            if changed_file.is_dir() {
                let canonical_dir = canonical_changed.as_ref().unwrap_or(changed_file);
                for (asset_path, routes) in &self.asset_to_routes {
                    if asset_path.starts_with(canonical_dir) {
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
