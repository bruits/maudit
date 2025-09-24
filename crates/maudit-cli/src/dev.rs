pub(crate) mod server;

mod filterer;

use colored::Colorize;
use filterer::should_watch_path;
use notify::{event::ModifyKind, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent};
use quanta::Instant;
use server::{update_status, WebSocketMessage};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::logging::{format_elapsed_time, FormatElapsedTimeOptions};

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

    // Do initial sync build
    info!(name: "build", "Doing initial build…");

    let child = std::process::Command::new("cargo")
        .args(["run", "--quiet"])
        .envs([
            ("MAUDIT_DEV", "true"),
            ("MAUDIT_QUIET", "true"),
            ("CARGO_TERM_COLOR", "always"),
            ("RUSTFLAGS", "-Awarnings"),
        ])
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Start a timer task to show warning after X seconds
    let warning_task = tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; // Adjust timeout as needed
        info!(name: "build", "{}", "This can take some time on the first run, or if there are uncached dependencies or assets..".dimmed());
    });

    // Wait for the command to finish
    let output = child.wait_with_output().unwrap();

    // Cancel the warning task since the command finished
    warning_task.abort();

    let stderr = String::from_utf8_lossy(&output.stderr);

    let duration = start_time.elapsed();
    let formatted_elasped_time =
        format_elapsed_time(duration, &FormatElapsedTimeOptions::default_dev());

    if output.status.success() {
        info!(name: "build", "Initial build finished {}", formatted_elasped_time);
    } else {
        error!(name: "build", "{}", stderr);
        error!(name: "build", "Initial build failed with errors {}", formatted_elasped_time);
    }

    let (sender_websocket, _) = broadcast::channel::<WebSocketMessage>(100);

    // Create shared status state
    let current_status = Arc::new(tokio::sync::RwLock::new(if !output.status.success() {
        Some(stderr.to_string())
    } else {
        None
    }));

    // Track the current build's cancellation token
    let current_build_cancel =
        Arc::new(tokio::sync::RwLock::new(Option::<CancellationToken>::None));

    let web_server_thread: tokio::task::JoinHandle<()> =
        tokio::spawn(server::start_dev_web_server(
            start_time,
            sender_websocket.clone(),
            host,
            if !output.status.success() {
                Some(stderr.to_string())
            } else {
                None
            },
            current_status.clone(),
        ));

    // Set up file watching with debouncer
    let (tx, mut rx) = tokio::sync::mpsc::channel::<DebounceEventResult>(100);

    let mut debouncer = new_debouncer(
        std::time::Duration::from_millis(100),
        None,
        move |result: DebounceEventResult| {
            tx.blocking_send(result).unwrap_or(());
        },
    )?;

    debouncer
        .watcher()
        .watch(Path::new(cwd), RecursiveMode::Recursive)?;

    // Handle file events
    tokio::spawn(async move {
        let browser_websocket = sender_websocket.clone();
        let current_status = current_status.clone();
        let build_cancel_ref = current_build_cancel.clone();

        while let Some(result) = rx.recv().await {
            match result {
                Ok(events) => {
                    // Filter events that should trigger a rebuild
                    let triggering_events: Vec<_> = events
                        .iter()
                        .filter(|event| should_rebuild_for_event(event))
                        .collect();

                    if !triggering_events.is_empty() {
                        debug!("File events: {} valid changes", triggering_events.len());
                        for event in &triggering_events {
                            for path in &event.paths {
                                debug!("  {:?}: {}", event.kind, path.display());
                            }
                        }

                        info!(name: "build", "Detected changes. Rebuilding…");

                        // Cancel any ongoing build
                        {
                            let mut current_cancel = build_cancel_ref.write().await;
                            if let Some(cancel_token) = current_cancel.take() {
                                cancel_token.cancel();
                                debug!("Cancelled previous build");
                            }
                        }

                        // Create new cancellation token for this build
                        let new_cancel_token = CancellationToken::new();
                        {
                            let mut current_cancel = build_cancel_ref.write().await;
                            *current_cancel = Some(new_cancel_token.clone());
                        }

                        let start_time = Instant::now();

                        // Run the build command
                        // TODO: Right now we always run `cargo run`, but for the sake of performance, we should detect in advance
                        // if the change even needs a full rebuild (e.g. if only content files changed, we can skip rebuilding the Rust binary)
                        // Perhaps this could be done by parsing the `.d` files that cargo generates.
                        let child = std::process::Command::new("cargo")
                            .args(["run", "--quiet"])
                            .envs([
                                ("MAUDIT_DEV", "true"),
                                ("MAUDIT_QUIET", "true"),
                                ("CARGO_TERM_COLOR", "always"),
                                ("RUSTFLAGS", "-Awarnings"),
                            ])
                            .stdout(std::process::Stdio::inherit())
                            .stderr(std::process::Stdio::piped())
                            .spawn();

                        match child {
                            Ok(child_process) => {
                                // Spawn the build in a separate task so we can cancel it
                                let build_task = tokio::task::spawn_blocking(move || {
                                    child_process.wait_with_output()
                                });

                                // Wait for either process completion or cancellation
                                let output_result = tokio::select! {
                                    output = build_task => {
                                        match output {
                                            Ok(result) => Some(result),
                                            Err(e) => {
                                                error!(name: "build", "Failed to join build task: {}", e);
                                                None
                                            }
                                        }
                                    }
                                    _ = new_cancel_token.cancelled() => {
                                        debug!("Build was cancelled by new file changes");
                                        None
                                    }
                                };

                                // Clear the cancellation token since build is done/cancelled
                                {
                                    let mut current_cancel = build_cancel_ref.write().await;
                                    *current_cancel = None;
                                }

                                if let Some(output) = output_result {
                                    match output {
                                        Ok(output) => {
                                            let duration = start_time.elapsed();
                                            let formatted_elapsed_time = format_elapsed_time(
                                                duration,
                                                &FormatElapsedTimeOptions::default_dev(),
                                            );

                                            if output.status.success() {
                                                info!(name: "build", "Rebuild finished {}", formatted_elapsed_time);

                                                // Update status and send success message to browser
                                                let websocket = browser_websocket.clone();
                                                let status = current_status.clone();
                                                tokio::spawn(async move {
                                                    update_status(
                                                        &websocket, status, "success", "",
                                                    )
                                                    .await;
                                                });
                                            } else {
                                                // TODO: It'd be great to somehow be able to get structured errors here (and in the initial build)
                                                // You can get some sort of structured errors from cargo with `--message-format=json`, but:
                                                // - You get an absurd amount of output, including non-error messages, at least when running `cargo run`
                                                // - You don't get the normal human-friendly output anymore, which would be great to have still
                                                //  - You can print the rendered output to the console from the JSON, but then you don't have colors
                                                // - It'd only work for rustc errors, not sure how we'd make it work with runtime errors.
                                                // ... So until then, we just send the raw stderr output and hopefully the user can make sense of it.
                                                let stderr =
                                                    String::from_utf8_lossy(&output.stderr)
                                                        .to_string();
                                                error!(name: "build", "{}", stderr);
                                                error!(name: "build", "Rebuild failed with errors {}", formatted_elapsed_time);

                                                // Update status and send error message to browser
                                                let websocket = browser_websocket.clone();
                                                let status = current_status.clone();
                                                tokio::spawn(async move {
                                                    update_status(
                                                        &websocket, status, "error", &stderr,
                                                    )
                                                    .await;
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            error!(name: "build", "Failed to wait for build process: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!(name: "build", "Failed to spawn build process: {}", e);
                            }
                        }
                    }
                }
                Err(errors) => {
                    for error in errors {
                        error!("File watch error: {:?}", error);
                    }
                }
            }
        }
    });

    // Wait for the web server to finish (this will run indefinitely)
    web_server_thread.await.unwrap();

    Ok(())
}
