use cargo_metadata::Message;
use quanta::Instant;
use server::{StatusType, WebSocketMessage, update_status};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::{
    dev::server,
    logging::{FormatElapsedTimeOptions, format_elapsed_time},
};

#[derive(Clone)]
pub struct BuildManager {
    current_cancel: Arc<tokio::sync::RwLock<Option<CancellationToken>>>,
    build_semaphore: Arc<tokio::sync::Semaphore>,
    websocket_tx: broadcast::Sender<WebSocketMessage>,
    current_status: Arc<tokio::sync::RwLock<Option<server::PersistentStatus>>>,
}

impl BuildManager {
    pub fn new(websocket_tx: broadcast::Sender<WebSocketMessage>) -> Self {
        Self {
            current_cancel: Arc::new(tokio::sync::RwLock::new(None)),
            build_semaphore: Arc::new(tokio::sync::Semaphore::new(1)), // Only one build at a time
            websocket_tx,
            current_status: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Get a reference to the current status for use with the web server
    pub fn current_status(&self) -> Arc<tokio::sync::RwLock<Option<server::PersistentStatus>>> {
        self.current_status.clone()
    }

    /// Do initial build that can be cancelled (but isn't stored as current build)
    pub async fn do_initial_build(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.internal_build(true).await
    }

    /// Start a new build, cancelling any previous one
    pub async fn start_build(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.internal_build(false).await
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
        update_status(
            &self.websocket_tx,
            self.current_status.clone(),
            StatusType::Info,
            "Building...",
        )
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

        let websocket_tx = self.websocket_tx.clone();
        let current_status = self.current_status.clone();
        let build_start_time = Instant::now();

        // Create a channel to get the build result back
        let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<bool>(1);

        // Spawn watcher task to monitor the child process
        tokio::spawn(async move {
            let output_future = async {
                // Read stdout concurrently with waiting for process to finish
                let stdout_task = tokio::spawn(async move {
                    let mut out = Vec::new();
                    tokio::io::copy(&mut stdout, &mut out).await.unwrap_or(0);

                    let mut rendered_messages: Vec<String> = Vec::new();

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
                                    // Random text came in, just log it
                                    Message::TextLine(msg) => {
                                        info!("{}", msg);
                                    }
                                    _ => {}
                                }
                            }
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
                let stdout_data = stdout_task.await.unwrap_or_default();
                let stderr_data = stderr_task.await.unwrap_or_default();

                Ok::<(std::process::Output, Vec<String>), Box<dyn std::error::Error + Send + Sync>>(
                    (
                        std::process::Output {
                            status,
                            stdout: stdout_data.0,
                            stderr: stderr_data,
                        },
                        stdout_data.1,
                    ),
                )
            };

            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!(name: "build", "Build cancelled");
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
                            let (output, rendered_messages) = output;
                            if output.status.success() {
                                let build_type = if is_initial { "Initial build" } else { "Rebuild" };
                                info!(name: "build", "{} finished {}", build_type, formatted_elapsed_time);
                                update_status(&websocket_tx, current_status, StatusType::Success, "Build finished successfully").await;
                                true
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                println!("{}", stderr); // Raw stderr sometimes has something to say whenever cargo fails, even if the errors messages are actually in stdout
                                let build_type = if is_initial { "Initial build" } else { "Rebuild" };
                                error!(name: "build", "{} failed with errors {}", build_type, formatted_elapsed_time);
                                if is_initial {
                                    error!(name: "build", "Initial build needs to succeed before we can start the dev server");
                                    update_status(&websocket_tx, current_status, StatusType::Error, "Initial build failed - fix errors and save to retry").await;
                                } else {
                                    update_status(&websocket_tx, current_status, StatusType::Error, &rendered_messages.join("\n")).await;
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

        // Wait for the build result
        let success = result_rx.recv().await.unwrap_or(false);
        Ok(success)
    }
}
