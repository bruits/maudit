use cargo_metadata::Message;
use quanta::Instant;
use server::{PersistentStatus, StatusManager, StatusType};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::{
    dev::server,
    logging::{FormatElapsedTimeOptions, format_elapsed_time},
};

use super::dep_tracker::DependencyTracker;

#[derive(Clone)]
pub struct BuildManager {
    current_cancel: Arc<RwLock<Option<CancellationToken>>>,
    build_semaphore: Arc<tokio::sync::Semaphore>,
    status_manager: StatusManager,
    /// Cached binary path after successful build
    binary_path: Arc<RwLock<Option<PathBuf>>>,
    /// Dependency tracker for determining if recompilation is needed
    dep_tracker: Arc<RwLock<Option<DependencyTracker>>>,
    /// Cached binary name (extracted from Cargo.toml or first successful build)
    binary_name: Arc<RwLock<Option<String>>>,
}

impl BuildManager {
    pub fn new(status_manager: StatusManager) -> Self {
        Self {
            current_cancel: Arc::new(RwLock::new(None)),
            build_semaphore: Arc::new(tokio::sync::Semaphore::new(1)), // Only one build at a time
            status_manager,
            binary_path: Arc::new(RwLock::new(None)),
            dep_tracker: Arc::new(RwLock::new(None)),
            binary_name: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a reference to the current status for use with the web server
    pub fn current_status(&self) -> Arc<RwLock<Option<PersistentStatus>>> {
        self.status_manager.current_status()
    }

    /// Get the status manager's broadcast sender for the web server
    pub fn websocket_sender(&self) -> tokio::sync::broadcast::Sender<server::WebSocketMessage> {
        self.status_manager.sender()
    }

    /// Do initial build that can be cancelled (but isn't stored as current build)
    pub async fn do_initial_build(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.internal_build(true).await
    }

    /// Start a new build, cancelling any previous one
    pub async fn start_build(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.internal_build(false).await
    }

    /// Check if the given changed paths require recompilation
    /// Returns true if recompilation is needed, false if we can just rerun the binary
    pub async fn needs_recompile(&self, changed_paths: &[PathBuf]) -> bool {
        let tracker = self.dep_tracker.read().await;
        match tracker.as_ref() {
            Some(tracker) => {
                let needs = tracker.needs_recompile(changed_paths);
                debug!(
                    "Dependency tracker says recompile needed: {} for {} changed files",
                    needs,
                    changed_paths.len()
                );
                needs
            }
            None => {
                // No dependency tracker yet, need to recompile
                debug!("No dependency tracker available, defaulting to recompile");
                true
            }
        }
    }

    /// Rerun the cached binary without recompilation
    /// This is used when only non-Rust files changed
    pub async fn rerun_binary(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let binary_path = {
            let path = self.binary_path.read().await;
            path.clone()
        };

        let Some(binary_path) = binary_path else {
            warn!("No cached binary path, falling back to full build");
            return self.start_build().await;
        };

        if !binary_path.exists() {
            warn!(
                "Cached binary no longer exists at {:?}, falling back to full build",
                binary_path
            );
            return self.start_build().await;
        }

        // Cancel any existing build/run immediately
        let cancel = CancellationToken::new();
        {
            let mut current_cancel = self.current_cancel.write().await;
            if let Some(old_cancel) = current_cancel.replace(cancel.clone()) {
                old_cancel.cancel();
            }
        }

        // Acquire semaphore to ensure only one build/run happens at a time
        let _ = self.build_semaphore.acquire().await?;

        // Notify that we're rerunning
        self.status_manager
            .update(StatusType::Info, "Rerunning...")
            .await;

        let build_start_time = Instant::now();

        info!(name: "build", "Rerunning binary (no recompilation needed)...");

        let mut child = Command::new(&binary_path)
            .envs([("MAUDIT_DEV", "true"), ("MAUDIT_QUIET", "true")])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        let status_manager = self.status_manager.clone();

        // Create a channel to get the result back
        let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<bool>(1);

        tokio::spawn(async move {
            let output_future = async {
                let stdout_task = tokio::spawn(async move {
                    let mut out = Vec::new();
                    tokio::io::copy(&mut stdout, &mut out).await.unwrap_or(0);
                    out
                });

                let stderr_task = tokio::spawn(async move {
                    let mut err = Vec::new();
                    tokio::io::copy(&mut stderr, &mut err).await.unwrap_or(0);
                    err
                });

                let status = child.wait().await?;
                let stdout_data = stdout_task.await.unwrap_or_default();
                let stderr_data = stderr_task.await.unwrap_or_default();

                Ok::<std::process::Output, Box<dyn std::error::Error + Send + Sync>>(
                    std::process::Output {
                        status,
                        stdout: stdout_data,
                        stderr: stderr_data,
                    },
                )
            };

            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!(name: "build", "Rerun cancelled");
                    let _ = child.kill().await;
                    status_manager.update(StatusType::Info, "Rerun cancelled").await;
                    let _ = result_tx.send(false).await;
                }
                res = output_future => {
                    let duration = build_start_time.elapsed();
                    let formatted_elapsed_time = format_elapsed_time(
                        duration,
                        &FormatElapsedTimeOptions::default_dev(),
                    );

                    let success = match res {
                        Ok(output) => {
                            if output.status.success() {
                                info!(name: "build", "Rerun finished {}", formatted_elapsed_time);
                                status_manager.update(StatusType::Success, "Build finished successfully").await;
                                true
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                if !stderr.is_empty() {
                                    println!("{}", stderr);
                                }
                                error!(name: "build", "Rerun failed {}", formatted_elapsed_time);
                                status_manager.update(StatusType::Error, &stderr).await;
                                false
                            }
                        }
                        Err(e) => {
                            error!(name: "build", "Failed to wait for rerun: {}", e);
                            status_manager.update(StatusType::Error, &format!("Failed to wait for rerun: {}", e)).await;
                            false
                        }
                    };
                    let _ = result_tx.send(success).await;
                }
            }
        });

        let success = result_rx.recv().await.unwrap_or(false);
        Ok(success)
    }

    /// Update the dependency tracker after a successful build
    async fn update_dependency_tracker(&self) {
        let binary_name = {
            let name = self.binary_name.read().await;
            name.clone()
        };

        let Some(binary_name) = binary_name else {
            debug!("No binary name cached, skipping dependency tracker update");
            return;
        };

        match DependencyTracker::load_from_binary_name(&binary_name) {
            Ok(tracker) => {
                let dep_count = tracker.get_dependencies().len();
                let mut dep_tracker = self.dep_tracker.write().await;
                *dep_tracker = Some(tracker);
                debug!(
                    "Updated dependency tracker with {} dependencies",
                    dep_count
                );
            }
            Err(e) => {
                warn!("Failed to load dependency tracker: {}", e);
            }
        }
    }

    /// Internal build method that handles both initial and regular builds
    async fn internal_build(&self, is_initial: bool) -> Result<bool, Box<dyn std::error::Error>> {
        // Cancel any existing build immediately
        let cancel = CancellationToken::new();
        {
            let mut current_cancel = self.current_cancel.write().await;
            if let Some(old_cancel) = current_cancel.replace(cancel.clone()) {
                old_cancel.cancel();
            }
        }

        // Acquire semaphore to ensure only one build runs at a time
        // This prevents resource conflicts if cancellation fails
        let _ = self.build_semaphore.acquire().await?;

        // Notify that build is starting
        self.status_manager
            .update(StatusType::Info, "Building...")
            .await;

        let mut child = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--message-format",
                "json-diagnostic-rendered-ansi",
            ])
            .envs([
                ("MAUDIT_DEV", "true"),
                ("MAUDIT_QUIET", "true"),
                ("CARGO_TERM_COLOR", "always"),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Take the stderr stream for manual handling
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        let status_manager = self.status_manager.clone();
        let build_start_time = Instant::now();

        // Create a channel to get the build result back (success, Option<binary_path>, Option<binary_name>)
        let (result_tx, mut result_rx) =
            tokio::sync::mpsc::channel::<(bool, Option<PathBuf>, Option<String>)>(1);

        // Spawn watcher task to monitor the child process
        tokio::spawn(async move {
            let output_future = async {
                // Read stdout concurrently with waiting for process to finish
                let stdout_task = tokio::spawn(async move {
                    let mut out = Vec::new();
                    tokio::io::copy(&mut stdout, &mut out).await.unwrap_or(0);

                    let mut rendered_messages: Vec<String> = Vec::new();
                    let mut binary_path: Option<PathBuf> = None;
                    let mut binary_name: Option<String> = None;

                    // Ideally we'd stream things as they come, but I can't figure it out
                    for message in cargo_metadata::Message::parse_stream(
                        String::from_utf8_lossy(&out).to_string().as_bytes(),
                    ) {
                        match message {
                            Err(e) => {
                                error!(name: "build", "Failed to parse cargo message: {}", e);
                                continue;
                            }
                            Ok(message) => {
                                match message {
                                    // Compiler wants to tell us something
                                    Message::CompilerMessage(msg) => {
                                        // TODO: For now, just send through the rendered messages, but in the future let's send
                                        // structured messages to the frontend so we can do better formatting
                                        if let Some(rendered) = &msg.message.rendered {
                                            info!("{}", rendered);
                                            rendered_messages.push(rendered.to_string());
                                        }
                                    }
                                    // Binary artifact produced - capture the path
                                    Message::CompilerArtifact(artifact) => {
                                        if artifact.executable.is_some() {
                                            binary_path =
                                                artifact.executable.map(|p| p.into_std_path_buf());
                                            binary_name = Some(artifact.target.name.clone());
                                            debug!(
                                                "Found binary artifact: {:?} ({})",
                                                binary_path, artifact.target.name
                                            );
                                        }
                                    }
                                    // Random text came in, just log it
                                    Message::TextLine(msg) => {
                                        info!("{}", msg);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    (out, rendered_messages, binary_path, binary_name)
                });

                let stderr_task = tokio::spawn(async move {
                    let mut err = Vec::new();
                    tokio::io::copy(&mut stderr, &mut err).await.unwrap_or(0);

                    err
                });

                let status = child.wait().await?;
                let stdout_data = stdout_task.await.unwrap_or_default();
                let stderr_data = stderr_task.await.unwrap_or_default();

                Ok::<
                    (std::process::Output, Vec<String>, Option<PathBuf>, Option<String>),
                    Box<dyn std::error::Error + Send + Sync>,
                >((
                    std::process::Output {
                        status,
                        stdout: stdout_data.0,
                        stderr: stderr_data,
                    },
                    stdout_data.1,
                    stdout_data.2,
                    stdout_data.3,
                ))
            };

            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!(name: "build", "Build cancelled");
                    let _ = child.kill().await;
                    status_manager.update(StatusType::Info, "Build cancelled").await;
                    let _ = result_tx.send((false, None, None)).await; // Build failed due to cancellation
                }
                res = output_future => {
                    let duration = build_start_time.elapsed();
                    let formatted_elapsed_time = format_elapsed_time(
                        duration,
                        &FormatElapsedTimeOptions::default_dev(),
                    );

                    let (success, binary_path, binary_name) = match res {
                        Ok(output) => {
                            let (output, rendered_messages, binary_path, binary_name) = output;
                            if output.status.success() {
                                let build_type = if is_initial { "Initial build" } else { "Rebuild" };
                                info!(name: "build", "{} finished {}", build_type, formatted_elapsed_time);
                                status_manager.update(StatusType::Success, "Build finished successfully").await;
                                (true, binary_path, binary_name)
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                println!("{}", stderr); // Raw stderr sometimes has something to say whenever cargo fails, even if the errors messages are actually in stdout
                                let build_type = if is_initial { "Initial build" } else { "Rebuild" };
                                error!(name: "build", "{} failed with errors {}", build_type, formatted_elapsed_time);
                                if is_initial {
                                    error!(name: "build", "Initial build needs to succeed before we can start the dev server");
                                    status_manager.update(StatusType::Error, "Initial build failed - fix errors and save to retry").await;
                                } else {
                                    status_manager.update(StatusType::Error, &rendered_messages.join("\n")).await;
                                }
                                (false, None, None)
                            }
                        }
                        Err(e) => {
                            error!(name: "build", "Failed to wait for build: {}", e);
                            status_manager.update(StatusType::Error, &format!("Failed to wait for build: {}", e)).await;
                            (false, None, None)
                        }
                    };
                    let _ = result_tx.send((success, binary_path, binary_name)).await;
                }
            }
        });

        // Wait for the build result
        let (success, binary_path, binary_name) = result_rx.recv().await.unwrap_or((false, None, None));

        // Cache the binary path and name if we got them
        if let Some(path) = binary_path {
            debug!("Caching binary path: {:?}", path);
            let mut cached_path = self.binary_path.write().await;
            *cached_path = Some(path);
        }

        if let Some(name) = binary_name {
            debug!("Caching binary name: {}", name);
            let mut cached_name = self.binary_name.write().await;
            *cached_name = Some(name);
        }

        // Update dependency tracker after successful build
        if success {
            self.update_dependency_tracker().await;
        }

        Ok(success)
    }
}
