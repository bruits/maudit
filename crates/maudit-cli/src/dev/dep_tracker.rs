use depinfo::RustcDepInfo;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, warn};

/// Tracks dependencies from .d files to determine if recompilation is needed
#[derive(Debug, Clone)]
pub struct DependencyTracker {
    /// Path to the .d file
    pub(crate) d_file_path: Option<PathBuf>,
    /// Map of dependency paths to their last modification times
    pub(crate) dependencies: HashMap<PathBuf, SystemTime>,
}

/// Find the target/debug directory.
///
/// Uses `CARGO_TARGET_DIR` if set, otherwise walks up from the current
/// directory looking for `Cargo.lock` (the workspace/project root) and
/// appends `target`. Falls back to a relative `target` path.
pub fn find_target_dir() -> Result<PathBuf, std::io::Error> {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| find_project_root().join("target"));

    let debug_dir = target_dir.join("debug");
    debug!("Using target directory: {:?}", debug_dir);
    Ok(debug_dir)
}

/// Find the project/workspace root by walking up from the current directory
/// looking for `Cargo.lock`.
fn find_project_root() -> PathBuf {
    let Ok(mut current) = std::env::current_dir() else {
        return PathBuf::from(".");
    };

    for _ in 0..5 {
        if current.join("Cargo.lock").exists() {
            return current;
        }
        if !current.pop() {
            break;
        }
    }

    PathBuf::from(".")
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

    /// Reload dependencies from the .d file using the depinfo crate
    pub fn reload_dependencies(&mut self) -> Result<(), std::io::Error> {
        let Some(d_file_path) = &self.d_file_path else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No .d file path set",
            ));
        };

        let dep_info = RustcDepInfo::from_file(d_file_path).map_err(|e| {
            warn!("Failed to parse .d file at {:?}: {}", d_file_path, e);
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        // Clear old dependencies and load new ones with their modification times
        self.dependencies.clear();

        for dep_path in dep_info.files {
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
                            && current_modified > *last_modified
                        {
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

    #[test]
    fn test_parse_d_file_with_spaces() {
        let temp_dir = TempDir::new().unwrap();
        let d_file_path = temp_dir.path().join("test_spaces.d");

        // Create actual test files with spaces in names
        let dep_with_space = temp_dir.path().join("my file.rs");
        fs::write(&dep_with_space, "// test").unwrap();

        let normal_dep = temp_dir.path().join("normal.rs");
        fs::write(&normal_dep, "// test").unwrap();

        // Create a mock .d file with escaped spaces (Make format)
        let mut d_file = fs::File::create(&d_file_path).unwrap();
        writeln!(
            d_file,
            "/path/to/target: {} {}",
            dep_with_space.to_str().unwrap().replace(' ', "\\ "),
            normal_dep.to_str().unwrap()
        )
        .unwrap();

        let mut tracker = DependencyTracker::new();
        tracker.d_file_path = Some(d_file_path);

        // Load dependencies
        tracker.reload_dependencies().unwrap();

        // Should have successfully parsed both files
        let deps = tracker.get_dependencies();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|p| p.to_str().unwrap().contains("my file.rs")),
            "Should contain file with space"
        );
        assert!(
            deps.iter()
                .any(|p| p.to_str().unwrap().contains("normal.rs")),
            "Should contain normal file"
        );
    }
}
