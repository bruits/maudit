use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, warn};

/// Tracks dependencies from .d files to determine if recompilation is needed
#[derive(Debug, Clone)]
pub struct DependencyTracker {
    /// Path to the .d file
    d_file_path: Option<PathBuf>,
    /// Map of dependency paths to their last modification times
    dependencies: HashMap<PathBuf, SystemTime>,
}

/// Find the target directory using multiple strategies
///
/// This function tries multiple approaches to locate the target directory:
/// 1. CARGO_TARGET_DIR / CARGO_BUILD_TARGET_DIR environment variables
/// 2. Local ./target/debug directory
/// 3. Workspace root target/debug directory (walking up to find [workspace])
/// 4. Fallback to relative "target/debug" path
pub fn find_target_dir() -> Result<PathBuf, std::io::Error> {
    // 1. Check CARGO_TARGET_DIR and CARGO_BUILD_TARGET_DIR environment variables
    for env_var in ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"] {
        if let Ok(target_dir) = std::env::var(env_var) {
            // Try with /debug appended
            let path = PathBuf::from(&target_dir).join("debug");
            if path.exists() {
                debug!("Using target directory from {}: {:?}", env_var, path);
                return Ok(path);
            }
            // If the env var points directly to debug or release
            let path_no_debug = PathBuf::from(&target_dir);
            if path_no_debug.exists()
                && (path_no_debug.ends_with("debug") || path_no_debug.ends_with("release"))
            {
                debug!(
                    "Using target directory from {} (direct): {:?}",
                    env_var, path_no_debug
                );
                return Ok(path_no_debug);
            }
        }
    }

    // 2. Look for target directory in current directory
    let local_target = PathBuf::from("target/debug");
    if local_target.exists() {
        debug!("Using local target directory: {:?}", local_target);
        return Ok(local_target);
    }

    // 3. Try to find workspace root by looking for Cargo.toml with [workspace]
    let mut current = std::env::current_dir()?;
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
                && content.contains("[workspace]") {
                    let workspace_target = current.join("target").join("debug");
                    if workspace_target.exists() {
                        debug!("Using workspace target directory: {:?}", workspace_target);
                        return Ok(workspace_target);
                    }
                }

        // Move up to parent directory
        if !current.pop() {
            break;
        }
    }

    // 4. Final fallback to relative path
    debug!("Falling back to relative target/debug path");
    Ok(PathBuf::from("target/debug"))
}

impl DependencyTracker {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            d_file_path: None,
            dependencies: HashMap::new(),
        }
    }

    /// Locate and load the .d file for the current binary
    /// The .d file is typically at target/debug/<binary-name>.d
    pub fn load_from_binary_name(binary_name: &str) -> Result<Self, std::io::Error> {
        let target_dir = find_target_dir()?;
        let d_file_path = target_dir.join(format!("{}.d", binary_name));

        if !d_file_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(".d file not found at {:?}", d_file_path),
            ));
        }

        let mut tracker = Self {
            d_file_path: Some(d_file_path.clone()),
            dependencies: HashMap::new(),
        };

        tracker.reload_dependencies()?;
        Ok(tracker)
    }

    /// Reload dependencies from the .d file
    pub fn reload_dependencies(&mut self) -> Result<(), std::io::Error> {
        let Some(d_file_path) = &self.d_file_path else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No .d file path set",
            ));
        };

        let content = fs::read_to_string(d_file_path)?;

        // Parse the .d file format: "target: dep1 dep2 dep3 ..."
        // The first line contains the target and dependencies, separated by ':'
        let deps = if let Some(colon_pos) = content.find(':') {
            // Everything after the colon is dependencies
            &content[colon_pos + 1..]
        } else {
            // Malformed .d file
            warn!("Malformed .d file at {:?}", d_file_path);
            return Ok(());
        };

        // Dependencies are space-separated and may span multiple lines (with line continuations)
        let dep_paths: Vec<PathBuf> = deps
            .split_whitespace()
            .filter(|s| !s.is_empty() && *s != "\\") // Filter out line continuation characters
            .map(PathBuf::from)
            .collect();

        // Clear old dependencies and load new ones with their modification times
        self.dependencies.clear();

        for dep_path in dep_paths {
            match fs::metadata(&dep_path) {
                Ok(metadata) => {
                    if let Ok(modified) = metadata.modified() {
                        self.dependencies.insert(dep_path.clone(), modified);
                        debug!("Tracking dependency: {:?}", dep_path);
                    }
                }
                Err(e) => {
                    // Dependency file doesn't exist or can't be read - this is okay,
                    // it might have been deleted or moved
                    debug!("Could not read dependency {:?}: {}", dep_path, e);
                }
            }
        }

        debug!(
            "Loaded {} dependencies from {:?}",
            self.dependencies.len(),
            d_file_path
        );
        Ok(())
    }

    /// Check if any of the given paths require recompilation
    /// Returns true if any path is a tracked dependency that has been modified
    pub fn needs_recompile(&self, changed_paths: &[PathBuf]) -> bool {
        for changed_path in changed_paths {
            // Normalize the changed path to handle relative vs absolute paths
            let changed_path_canonical = changed_path.canonicalize().ok();

            for (dep_path, last_modified) in &self.dependencies {
                // Try to match both exact path and canonical path
                let matches = changed_path == dep_path
                    || changed_path_canonical.as_ref() == Some(dep_path)
                    || dep_path.canonicalize().ok().as_ref() == changed_path_canonical.as_ref();

                if matches {
                    // Check if the file was modified after we last tracked it
                    if let Ok(metadata) = fs::metadata(changed_path) {
                        if let Ok(current_modified) = metadata.modified()
                            && current_modified > *last_modified {
                                debug!(
                                    "Dependency {:?} was modified, recompile needed",
                                    changed_path
                                );
                                return true;
                            }
                    } else {
                        // File was deleted or can't be read, assume recompile is needed
                        debug!(
                            "Dependency {:?} no longer exists, recompile needed",
                            changed_path
                        );
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get the list of tracked dependency paths
    pub fn get_dependencies(&self) -> Vec<&Path> {
        self.dependencies.keys().map(|p| p.as_path()).collect()
    }

    /// Check if we have any dependencies loaded
    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_parse_d_file() {
        let temp_dir = TempDir::new().unwrap();
        let d_file_path = temp_dir.path().join("test.d");

        // Create a mock .d file
        let mut d_file = fs::File::create(&d_file_path).unwrap();
        writeln!(
            d_file,
            "/path/to/target: /path/to/dep1.rs /path/to/dep2.rs \\"
        )
        .unwrap();
        writeln!(d_file, "  /path/to/dep3.rs").unwrap();

        // Create a tracker and point it to our test file
        let mut tracker = DependencyTracker::new();
        tracker.d_file_path = Some(d_file_path);

        // This will fail to load the actual files, but we can check the parsing logic
        let _ = tracker.reload_dependencies();

        // We won't have any dependencies because the files don't exist,
        // but we've verified the parsing doesn't crash
    }
}
