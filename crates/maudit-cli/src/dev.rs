pub(crate) mod server;

mod filterer;

use filterer::should_watch_path;
use notify::{EventKind, RecursiveMode, WatchFilter, event::ModifyKind};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, new_debouncer};
use quanta::Instant;
use server::{StatusType, WebSocketMessage, update_status};
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::{Mutex, broadcast};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::logging::{FormatElapsedTimeOptions, format_elapsed_time};

struct BuildHandle {
    cancel: CancellationToken,
}

#[derive(Clone)]
struct BuildManager {
    current: Arc<Mutex<Option<BuildHandle>>>,
    websocket_tx: broadcast::Sender<WebSocketMessage>,
    current_status: Arc<tokio::sync::RwLock<Option<server::PersistentStatus>>>,
}

impl BuildManager {
    fn new(
        websocket_tx: broadcast::Sender<WebSocketMessage>,
        current_status: Arc<tokio::sync::RwLock<Option<server::PersistentStatus>>>,
    ) -> Self {
        Self {
            current: Arc::new(Mutex::new(None)),
            websocket_tx,
            current_status,
        }
    }

    /// Do initial build that can be cancelled (but isn't stored as current build)
    async fn do_initial_build(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.internal_build(true).await
    }

    /// Start a new build, cancelling any previous one
    async fn start_build(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.internal_build(false).await
    }

    /// Internal build method that handles both initial and regular builds
    async fn internal_build(&self, is_initial: bool) -> Result<bool, Box<dyn std::error::Error>> {
        let mut current = self.current.lock().await;

        // Cancel old build (for both initial and regular builds)
        if let Some(old) = current.take() {
            old.cancel.cancel();
        }

        // Notify that build is starting
        update_status(
            &self.websocket_tx,
            self.current_status.clone(),
            StatusType::Info,
            "Building...",
        )
        .await;

        let mut child = Command::new("cargo")
            .args(["run", "--quiet"])
            .envs([
                ("MAUDIT_DEV", "true"),
                ("MAUDIT_QUIET", "true"),
                ("CARGO_TERM_COLOR", "always"),
                ("RUSTFLAGS", "-Awarnings"),
            ])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Take the stderr stream for manual handling
        let mut stderr = child.stderr.take().unwrap();

        let cancel = CancellationToken::new();
        let cancel_child = cancel.clone();
        let websocket_tx = self.websocket_tx.clone();
        let current_status = self.current_status.clone();
        let build_start_time = Instant::now();

        // Create a channel to get the build result back
        let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<bool>(1);

        // Spawn watcher task to monitor the child process
        tokio::spawn(async move {
            let output_future = async {
                let status = child.wait().await?;
                let mut err = Vec::new();
                tokio::io::copy(&mut stderr, &mut err).await?;
                Ok::<std::process::Output, Box<dyn std::error::Error + Send + Sync>>(
                    std::process::Output {
                        status,
                        stdout: Vec::new(), // We inherit stdout, so it's empty
                        stderr: err,
                    },
                )
            };

            tokio::select! {
                _ = cancel_child.cancelled() => {
                    info!(name: "build", "Build cancelled");
                    let _ = child.kill().await;
                    update_status(&websocket_tx, current_status, StatusType::Info, "Build cancelled").await;
                    let _ = result_tx.send(false).await; // Build failed due to cancellation
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
                                let build_type = if is_initial { "Initial build" } else { "Rebuild" };
                                info!(name: "build", "{} finished {}", build_type, formatted_elapsed_time);
                                update_status(&websocket_tx, current_status, StatusType::Success, "Build finished successfully").await;
                                true
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                error!(name: "build", "{}", stderr);
                                let build_type = if is_initial { "Initial build" } else { "Rebuild" };
                                error!(name: "build", "{} failed with errors {}", build_type, formatted_elapsed_time);
                                if is_initial {
                                    error!(name: "build", "Initial build needs to succeed before we can start the dev server");
                                    update_status(&websocket_tx, current_status, StatusType::Error, "Initial build failed - fix errors and save to retry").await;
                                } else {
                                    update_status(&websocket_tx, current_status, StatusType::Error, &stderr.to_string()).await;
                                }
                                false
                            }
                        }
                        Err(e) => {
                            error!(name: "build", "Failed to wait for build: {}", e);
                            update_status(&websocket_tx, current_status, StatusType::Error, &format!("Failed to wait for build: {}", e)).await;
                            false
                        }
                    };
                    let _ = result_tx.send(success).await;
                }
            }
        });

        // Store the build handle for all builds (both initial and regular)
        *current = Some(BuildHandle { cancel });

        // Wait for the build result
        let success = result_rx.recv().await.unwrap_or(false);
        Ok(success)
    }
}

fn should_rebuild_for_event(event: &DebouncedEvent) -> bool {
    event.paths.iter().any(|path| {
        should_watch_path(path)
            && match event.kind {
                // Only rebuild on actual content modifications, not metadata changes
                EventKind::Modify(ModifyKind::Data(_)) => true,
                EventKind::Modify(ModifyKind::Name(_)) => true,
                EventKind::Modify(ModifyKind::Any) => true,
                EventKind::Modify(ModifyKind::Other) => true,
                // Skip metadata-only changes (permissions, timestamps, etc.)
                EventKind::Modify(ModifyKind::Metadata(_)) => false,
                // Include file creation and removal
                EventKind::Create(_) => true,
                EventKind::Remove(_) => true,
                // Skip other event types
                _ => false,
            }
    })
}

pub async fn start_dev_env(cwd: &str, host: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    info!(name: "dev", "Preparing dev environment…");

    let (sender_websocket, _) = broadcast::channel::<WebSocketMessage>(100);

    // Create shared status state
    let current_status = Arc::new(tokio::sync::RwLock::new(None::<server::PersistentStatus>));

    // Create build manager with WebSocket and status references
    let build_manager = BuildManager::new(sender_websocket.clone(), current_status.clone());

    // Do initial build
    info!(name: "build", "Doing initial build…");
    let initial_build_success = build_manager.do_initial_build().await?;

    // Set up file watching with debouncer
    let (tx, mut rx) = tokio::sync::mpsc::channel::<DebounceEventResult>(100);

    let mut debouncer = new_debouncer(
        std::time::Duration::from_millis(100),
        None,
        move |result: DebounceEventResult| {
            tx.blocking_send(result).unwrap_or(());
        },
    )?;

    debouncer.watch_filtered(
        Path::new(cwd),
        RecursiveMode::Recursive,
        WatchFilter::with_filter(Arc::new(|path| {
            !path.components().any(|component| {
                matches!(
                    component.as_os_str().to_str(),
                    Some("target" | ".git" | "node_modules" | "dist")
                )
            })
        })),
    )?;

    let mut web_server_thread: Option<tokio::task::JoinHandle<()>> = None;

    // If initial build succeeded, start web server immediately
    if initial_build_success {
        info!(name: "dev", "Starting web server...");
        web_server_thread = Some(tokio::spawn(server::start_dev_web_server(
            start_time,
            sender_websocket.clone(),
            host,
            None,
            current_status.clone(),
        )));
    }

    // Clone build manager for the file watcher task
    let build_manager_watcher = build_manager.clone();
    let sender_websocket_watcher = sender_websocket.clone();
    let current_status_watcher = current_status.clone();

    let file_watcher_task = tokio::spawn(async move {
        let mut dev_server_started = initial_build_success;

        while let Some(result) = rx.recv().await {
            match result {
                Ok(events) => {
                    // Check if any event should trigger a rebuild
                    println!("{:?}", events);
                    let should_rebuild = events.iter().any(should_rebuild_for_event);

                    if should_rebuild {
                        if !dev_server_started {
                            // Initial build failed, retry it
                            info!(name: "watch", "Files changed, retrying initial build...");
                            match build_manager_watcher.do_initial_build().await {
                                Ok(true) => {
                                    info!(name: "build", "Initial build succeeded! Starting web server...");
                                    dev_server_started = true;

                                    // Start web server now that initial build succeeded
                                    tokio::spawn(server::start_dev_web_server(
                                        start_time,
                                        sender_websocket_watcher.clone(),
                                        host,
                                        None,
                                        current_status_watcher.clone(),
                                    ));
                                }
                                Ok(false) => {
                                    // Still failing, continue waiting
                                }
                                Err(e) => {
                                    error!(name: "build", "Failed to retry initial build: {}", e);
                                }
                            }
                        } else {
                            // Normal rebuild
                            info!(name: "watch", "Files changed, rebuilding...");
                            match build_manager_watcher.start_build().await {
                                Ok(_) => {
                                    // Build completed (success or failure already logged)
                                }
                                Err(e) => {
                                    error!(name: "build", "Failed to start build: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(errors) => {
                    for error in errors {
                        error!(name: "watch", "Watch error: {}", error);
                    }
                }
            }
        }
    });

    // Wait for either the web server or the file watcher to finish
    if let Some(web_server) = web_server_thread {
        tokio::select! {
            _ = web_server => {},
            _ = file_watcher_task => {},
        }
    } else {
        // No web server started yet, just wait for file watcher
        // (it will start the web server when initial build succeeds)
        file_watcher_task.await.unwrap();
    }
    Ok(())
}
