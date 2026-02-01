use cargo_metadata::Message;
use quanta::Instant;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::{
    dev::server::{StatusManager, StatusType},
    logging::{FormatElapsedTimeOptions, format_elapsed_time},
};

use super::dep_tracker::{DependencyTracker, find_target_dir};

/// Internal state shared across all BuildManager handles.
struct BuildManagerState {
    current_cancel: RwLock<Option<CancellationToken>>,
    build_semaphore: tokio::sync::Semaphore,
    status_manager: StatusManager,
    dep_tracker: RwLock<Option<DependencyTracker>>,
    binary_path: RwLock<Option<PathBuf>>,
    // Cached values computed once at startup
    target_dir: Option<PathBuf>,
    binary_name: Option<String>,
}

/// Manages cargo build processes with cancellation support.
/// Cheap to clone - all clones share the same underlying state.
#[derive(Clone)]
pub struct BuildManager {
    state: Arc<BuildManagerState>,
}

impl BuildManager {
    pub fn new(status_manager: StatusManager) -> Self {
        // Try to determine target directory and binary name at startup
        let target_dir = find_target_dir().ok();
        let binary_name = Self::get_binary_name_from_cargo_toml().ok();

        if let Some(ref name) = binary_name {
            debug!(name: "build", "Detected binary name at startup: {}", name);
        }
        if let Some(ref dir) = target_dir {
            debug!(name: "build", "Using target directory: {:?}", dir);
        }

        Self {
            state: Arc::new(BuildManagerState {
                current_cancel: RwLock::new(None),
                build_semaphore: tokio::sync::Semaphore::new(1),
                status_manager,
                dep_tracker: RwLock::new(None),
                binary_path: RwLock::new(None),
                target_dir,
                binary_name,
            }),
        }
    }

    /// Check if the given paths require recompilation based on dependency tracking.
    /// Returns true if recompilation is needed, false if we can just rerun the binary.
    pub async fn needs_recompile(&self, changed_paths: &[PathBuf]) -> bool {
        let dep_tracker = self.state.dep_tracker.read().await;

        if let Some(tracker) = dep_tracker.as_ref()
            && tracker.has_dependencies()
        {
            let needs_recompile = tracker.needs_recompile(changed_paths);
            if !needs_recompile {
                debug!(name: "build", "Changed files are not dependencies, rerun binary without recompile");
            }
            return needs_recompile;
        }

        // If we don't have a dependency tracker yet, always recompile
        true
    }

    /// Rerun the binary without recompiling.
    pub async fn rerun_binary(
        &self,
        changed_paths: &[PathBuf],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Get binary path with limited lock scope
        let path = {
            let guard = self.state.binary_path.read().await;
            match guard.as_ref() {
                Some(p) if p.exists() => p.clone(),
                Some(p) => {
                    warn!(name: "build", "Binary at {:?} no longer exists, falling back to full rebuild", p);
                    return self.start_build(Some(changed_paths)).await;
                }
                None => {
                    warn!(name: "build", "No binary path available, falling back to full rebuild");
                    return self.start_build(Some(changed_paths)).await;
                }
            }
        };

        // Log that we're doing an incremental build
        debug!(name: "build", "Incremental build: {} files changed", changed_paths.len());
        debug!(name: "build", "Changed files: {:?}", changed_paths);
        debug!(name: "build", "Rerunning binary without recompilation...");

        self.state
            .status_manager
            .update(StatusType::Info, "Rerunning...")
            .await;

        let build_start_time = Instant::now();

        // Serialize changed paths to JSON for the binary
        let changed_files_json = serde_json::to_string(changed_paths)?;

        let child = Command::new(&path)
            .envs([
                ("MAUDIT_DEV", "true"),
                ("MAUDIT_QUIET", "true"),
                ("MAUDIT_CHANGED_FILES", changed_files_json.as_str()),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let output = child.wait_with_output().await?;

        let duration = build_start_time.elapsed();
        let formatted_elapsed_time =
            format_elapsed_time(duration, &FormatElapsedTimeOptions::default_dev());

        if output.status.success() {
            if std::env::var("MAUDIT_SHOW_BINARY_OUTPUT").is_ok() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stdout.lines().chain(stderr.lines()) {
                    if !line.trim().is_empty() {
                        info!(name: "build", "{}", line);
                    }
                }
            }
            info!(name: "build", "Binary rerun finished {}", formatted_elapsed_time);
            self.state
                .status_manager
                .update(StatusType::Success, "Binary rerun finished successfully")
                .await;
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            error!(name: "build", "Binary rerun failed {}\nstdout: {}\nstderr: {}",
                formatted_elapsed_time, stdout, stderr);
            self.state
                .status_manager
                .update(
                    StatusType::Error,
                    &format!("Binary rerun failed:\n{}\n{}", stdout, stderr),
                )
                .await;
            Ok(false)
        }
    }

    /// Do initial build that can be cancelled.
    pub async fn do_initial_build(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        self.internal_build(true, None).await
    }

    /// Start a new build, cancelling any previous one.
    /// If changed_paths is provided, they will be passed to the binary for incremental builds.
    pub async fn start_build(
        &self,
        changed_paths: Option<&[PathBuf]>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        self.internal_build(false, changed_paths).await
    }

    async fn internal_build(
        &self,
        is_initial: bool,
        changed_paths: Option<&[PathBuf]>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Cancel any existing build immediately
        let cancel = CancellationToken::new();
        {
            let mut current_cancel = self.state.current_cancel.write().await;
            if let Some(old_cancel) = current_cancel.replace(cancel.clone()) {
                old_cancel.cancel();
            }
        }

        // Acquire semaphore to ensure only one build runs at a time
        let _permit = self.state.build_semaphore.acquire().await?;

        self.state
            .status_manager
            .update(StatusType::Info, "Building...")
            .await;

        // Build environment variables
        let mut envs: Vec<(&str, String)> = vec![
            ("MAUDIT_DEV", "true".to_string()),
            ("MAUDIT_QUIET", "true".to_string()),
            ("CARGO_TERM_COLOR", "always".to_string()),
        ];

        // Add changed files if provided (for incremental builds after recompilation)
        if let Some(paths) = changed_paths
            && let Ok(json) = serde_json::to_string(paths) {
                debug!(name: "build", "Passing MAUDIT_CHANGED_FILES to cargo: {}", json);
                envs.push(("MAUDIT_CHANGED_FILES", json));
            }

        let mut child = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--message-format",
                "json-diagnostic-rendered-ansi",
            ])
            .envs(envs.iter().map(|(k, v)| (*k, v.as_str())))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Take stdout/stderr before select! so we can use them in the completion branch
        // while still being able to kill the child in the cancellation branch
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let build_start_time = Instant::now();

        tokio::select! {
            _ = cancel.cancelled() => {
                debug!(name: "build", "Build cancelled");
                let _ = child.kill().await;
                self.state.status_manager.update(StatusType::Info, "Build cancelled").await;
                Ok(false)
            }
            result = self.run_build_to_completion(&mut child, stdout, stderr, is_initial, build_start_time) => {
                result
            }
        }
    }

    /// Run the cargo build process to completion and handle the output.
    async fn run_build_to_completion(
        &self,
        child: &mut tokio::process::Child,
        mut stdout: tokio::process::ChildStdout,
        mut stderr: tokio::process::ChildStderr,
        is_initial: bool,
        build_start_time: Instant,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Read stdout and stderr concurrently
        let stdout_task = tokio::spawn(async move {
            let mut out = Vec::new();
            tokio::io::copy(&mut stdout, &mut out).await.unwrap_or(0);

            let mut rendered_messages: Vec<String> = Vec::new();

            for message in cargo_metadata::Message::parse_stream(
                String::from_utf8_lossy(&out).to_string().as_bytes(),
            ) {
                match message {
                    Err(e) => {
                        error!(name: "build", "Failed to parse cargo message: {}", e);
                    }
                    Ok(Message::CompilerMessage(msg)) => {
                        if let Some(rendered) = &msg.message.rendered {
                            info!("{}", rendered);
                            rendered_messages.push(rendered.to_string());
                        }
                    }
                    Ok(Message::TextLine(msg)) => {
                        info!("{}", msg);
                    }
                    _ => {}
                }
            }

            (out, rendered_messages)
        });

        let stderr_task = tokio::spawn(async move {
            let mut err = Vec::new();
            tokio::io::copy(&mut stderr, &mut err).await.unwrap_or(0);
            err
        });

        let status = child.wait().await?;
        let (_stdout_bytes, rendered_messages) = stdout_task.await.unwrap_or_default();
        let stderr_bytes = stderr_task.await.unwrap_or_default();

        let duration = build_start_time.elapsed();
        let formatted_elapsed_time =
            format_elapsed_time(duration, &FormatElapsedTimeOptions::default_dev());

        if status.success() {
            let build_type = if is_initial {
                "Initial build"
            } else {
                "Rebuild"
            };
            info!(name: "build", "{} finished {}", build_type, formatted_elapsed_time);
            self.state
                .status_manager
                .update(StatusType::Success, "Build finished successfully")
                .await;

            self.update_dependency_tracker().await;

            Ok(true)
        } else {
            let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();
            // Raw stderr sometimes has something to say whenever cargo fails
            println!("{}", stderr_str);

            let build_type = if is_initial {
                "Initial build"
            } else {
                "Rebuild"
            };
            error!(name: "build", "{} failed with errors {}", build_type, formatted_elapsed_time);

            if is_initial {
                error!(name: "build", "Initial build needs to succeed before we can start the dev server");
                self.state
                    .status_manager
                    .update(
                        StatusType::Error,
                        "Initial build failed - fix errors and save to retry",
                    )
                    .await;
            } else {
                self.state
                    .status_manager
                    .update(StatusType::Error, &rendered_messages.join("\n"))
                    .await;
            }

            Ok(false)
        }
    }

    /// Update the dependency tracker after a successful build.
    async fn update_dependency_tracker(&self) {
        let Some(ref name) = self.state.binary_name else {
            debug!(name: "build", "No binary name available, skipping dependency tracker update");
            return;
        };

        let Some(ref target) = self.state.target_dir else {
            debug!(name: "build", "No target directory available, skipping dependency tracker update");
            return;
        };

        // Update binary path
        let bin_path = target.join(name);
        if bin_path.exists() {
            *self.state.binary_path.write().await = Some(bin_path.clone());
            debug!(name: "build", "Binary path set to: {:?}", bin_path);
        } else {
            debug!(name: "build", "Binary not found at expected path: {:?}", bin_path);
        }

        // Reload the dependency tracker from the .d file
        match DependencyTracker::load_from_binary_name(name) {
            Ok(tracker) => {
                debug!(name: "build", "Loaded {} dependencies for tracking", tracker.get_dependencies().len());
                *self.state.dep_tracker.write().await = Some(tracker);
            }
            Err(e) => {
                debug!(name: "build", "Could not load dependency tracker: {}", e);
            }
        }
    }

    fn get_binary_name_from_cargo_toml() -> Result<String, Box<dyn std::error::Error + Send + Sync>>
    {
        let cargo_toml_path = PathBuf::from("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Err("Cargo.toml not found in current directory".into());
        }

        let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path)?;
        let cargo_toml: toml::Value = toml::from_str(&cargo_toml_content)?;

        if let Some(package_name) = cargo_toml
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        {
            // Check if there's a [[bin]] section with a different name
            if let Some(bins) = cargo_toml.get("bin").and_then(|b| b.as_array())
                && let Some(first_bin) = bins.first()
                && let Some(bin_name) = first_bin.get("name").and_then(|n| n.as_str())
            {
                return Ok(bin_name.to_string());
            }

            return Ok(package_name.to_string());
        }

        Err("Could not find package name in Cargo.toml".into())
    }

    /// Set the dependency tracker directly (for testing).
    #[cfg(test)]
    pub(crate) async fn set_dep_tracker(&self, tracker: Option<DependencyTracker>) {
        *self.state.dep_tracker.write().await = tracker;
    }

    /// Set the binary path directly (for testing).
    #[cfg(test)]
    pub(crate) async fn set_binary_path(&self, path: Option<PathBuf>) {
        *self.state.binary_path.write().await = path;
    }

    /// Get the current binary path (for testing).
    #[cfg(test)]
    pub(crate) async fn get_binary_path(&self) -> Option<PathBuf> {
        self.state.binary_path.read().await.clone()
    }

    /// Create a BuildManager with custom target_dir and binary_name (for testing).
    #[cfg(test)]
    pub(crate) fn new_with_config(
        status_manager: StatusManager,
        target_dir: Option<PathBuf>,
        binary_name: Option<String>,
    ) -> Self {
        Self {
            state: Arc::new(BuildManagerState {
                current_cancel: RwLock::new(None),
                build_semaphore: tokio::sync::Semaphore::new(1),
                status_manager,
                dep_tracker: RwLock::new(None),
                binary_path: RwLock::new(None),
                target_dir,
                binary_name,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn create_test_manager() -> BuildManager {
        let status_manager = StatusManager::new();
        BuildManager::new_with_config(status_manager, None, None)
    }

    fn create_test_manager_with_config(
        target_dir: Option<PathBuf>,
        binary_name: Option<String>,
    ) -> BuildManager {
        let status_manager = StatusManager::new();
        BuildManager::new_with_config(status_manager, target_dir, binary_name)
    }

    #[tokio::test]
    async fn test_build_manager_clone_shares_state() {
        let manager1 = create_test_manager();
        let manager2 = manager1.clone();

        // Set binary path via one clone
        let test_path = PathBuf::from("/test/path");
        manager1.set_binary_path(Some(test_path.clone())).await;

        // Should be visible via the other clone
        assert_eq!(manager2.get_binary_path().await, Some(test_path));
    }

    #[tokio::test]
    async fn test_needs_recompile_without_tracker() {
        let manager = create_test_manager();

        // Without a dependency tracker, should always return true
        let changed = vec![PathBuf::from("src/main.rs")];
        assert!(manager.needs_recompile(&changed).await);
    }

    #[tokio::test]
    async fn test_needs_recompile_with_empty_tracker() {
        let manager = create_test_manager();

        // Set an empty tracker (no dependencies)
        let tracker = DependencyTracker::new();
        manager.set_dep_tracker(Some(tracker)).await;

        // Empty tracker has no dependencies, so has_dependencies() returns false
        // This means we should still return true (recompile needed)
        let changed = vec![PathBuf::from("src/main.rs")];
        assert!(manager.needs_recompile(&changed).await);
    }

    #[tokio::test]
    async fn test_needs_recompile_with_matching_dependency() {
        let manager = create_test_manager();

        // Create a tracker with some dependencies
        let temp_dir = TempDir::new().unwrap();
        let dep_file = temp_dir.path().join("src/lib.rs");
        std::fs::create_dir_all(dep_file.parent().unwrap()).unwrap();
        std::fs::write(&dep_file, "// test").unwrap();

        // Get canonical path and current mod time
        let canonical_path = dep_file.canonicalize().unwrap();
        let old_time = SystemTime::UNIX_EPOCH; // Very old time

        let mut tracker = DependencyTracker::new();
        tracker.dependencies = HashMap::from([(canonical_path, old_time)]);

        manager.set_dep_tracker(Some(tracker)).await;

        // Changed file IS a dependency and is newer - should need recompile
        let changed = vec![dep_file];
        assert!(manager.needs_recompile(&changed).await);
    }

    #[tokio::test]
    async fn test_needs_recompile_with_non_matching_file() {
        let manager = create_test_manager();

        // Create a tracker with some dependencies
        let temp_dir = TempDir::new().unwrap();
        let dep_file = temp_dir.path().join("src/lib.rs");
        std::fs::create_dir_all(dep_file.parent().unwrap()).unwrap();
        std::fs::write(&dep_file, "// test").unwrap();

        let canonical_path = dep_file.canonicalize().unwrap();
        let mod_time = std::fs::metadata(&dep_file).unwrap().modified().unwrap();

        let mut tracker = DependencyTracker::new();
        tracker.dependencies = HashMap::from([(canonical_path, mod_time)]);

        manager.set_dep_tracker(Some(tracker)).await;

        // Changed file is NOT a dependency (different file)
        let other_file = temp_dir.path().join("assets/style.css");
        std::fs::create_dir_all(other_file.parent().unwrap()).unwrap();
        std::fs::write(&other_file, "/* css */").unwrap();

        let changed = vec![other_file];
        assert!(!manager.needs_recompile(&changed).await);
    }

    #[tokio::test]
    async fn test_update_dependency_tracker_with_config_missing_binary() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_test_manager_with_config(
            Some(temp_dir.path().to_path_buf()),
            Some("nonexistent-binary".to_string()),
        );

        // Binary doesn't exist, so binary_path should not be set
        manager.update_dependency_tracker().await;

        assert!(manager.get_binary_path().await.is_none());
    }

    #[tokio::test]
    async fn test_update_dependency_tracker_with_existing_binary() {
        let temp_dir = TempDir::new().unwrap();
        let binary_name = "test-binary";
        let binary_path = temp_dir.path().join(binary_name);

        // Create a fake binary file
        std::fs::write(&binary_path, "fake binary").unwrap();

        let manager = create_test_manager_with_config(
            Some(temp_dir.path().to_path_buf()),
            Some(binary_name.to_string()),
        );

        manager.update_dependency_tracker().await;

        // Binary path should be set
        assert_eq!(manager.get_binary_path().await, Some(binary_path));
    }
}
