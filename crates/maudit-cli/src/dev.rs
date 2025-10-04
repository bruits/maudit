pub(crate) mod server;

mod build;
mod filterer;

use notify::{EventKind, RecursiveMode, WatchFilter, event::ModifyKind};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, new_debouncer};
use quanta::Instant;
use server::WebSocketMessage;
use std::path::Path;
use std::sync::Arc;
use tokio::{
    sync::{broadcast, mpsc::channel},
    task::JoinHandle,
};
use tracing::{error, info};

use crate::dev::build::BuildManager;

pub async fn start_dev_env(cwd: &str, host: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    info!(name: "dev", "Preparing dev environment…");

    let (sender_websocket, _) = broadcast::channel::<WebSocketMessage>(100);

    // Create build manager (it will create its own status state internally)
    let build_manager = BuildManager::new(sender_websocket.clone());

    // Do initial build
    info!(name: "build", "Doing initial build…");
    let initial_build_success = build_manager.do_initial_build().await?;

    // Set up file watching with debouncer
    let (tx, mut rx) = channel::<DebounceEventResult>(100);

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
            build_manager.current_status(),
        )));
    }

    // Clone build manager for the file watcher task
    let build_manager_watcher = build_manager.clone();
    let sender_websocket_watcher = sender_websocket.clone();

    let file_watcher_task = tokio::spawn(async move {
        let mut dev_server_started = initial_build_success;
        let mut dev_server_handle: Option<JoinHandle<()>> = None;

        loop {
            tokio::select! {
                // Handle file system events
                result = rx.recv() => {
                    let Some(result) = result else {
                        break; // Channel closed
                    };

                    match result {
                        Ok(events) => {
                            let should_rebuild = events.iter().any(should_rebuild_for_event);

                            if should_rebuild {
                                if !dev_server_started {
                                    // Initial build failed, retry it
                                    info!(name: "watch", "Files changed, retrying initial build...");
                                    let start_time = Instant::now();
                                    match build_manager_watcher.do_initial_build().await {
                                        Ok(true) => {
                                            info!(name: "build", "Initial build succeeded! Starting web server...");
                                            dev_server_started = true;

                                            dev_server_handle =
                                                Some(tokio::spawn(server::start_dev_web_server(
                                                    start_time,
                                                    sender_websocket_watcher.clone(),
                                                    host,
                                                    None,
                                                    build_manager_watcher.current_status(),
                                                )));
                                        }
                                        Ok(false) => {
                                            // Still failing, continue waiting
                                        }
                                        Err(e) => {
                                            error!(name: "build", "Failed to retry initial build: {}", e);
                                        }
                                    }
                                } else {
                                    // Normal rebuild - spawn in background so file watcher can continue
                                    info!(name: "watch", "Files changed, rebuilding...");
                                    let build_manager_clone = build_manager_watcher.clone();
                                    tokio::spawn(async move {
                                        match build_manager_clone.start_build().await {
                                            Ok(_) => {
                                                // Build completed (success or failure already logged)
                                            }
                                            Err(e) => {
                                                error!(name: "build", "Failed to start build: {}", e);
                                            }
                                        }
                                    });
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
                // Monitor dev server - if it ends, file watcher ends too
                _ = async {
                    if let Some(handle) = &mut dev_server_handle {
                        handle.await
                    } else {
                        std::future::pending().await // Never resolves if no dev server
                    }
                } => {
                    break;
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
        // If it started the web server, it'll also close itself if the web server ends
        file_watcher_task.await.unwrap();
    }
    Ok(())
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

fn should_watch_path(path: &Path) -> bool {
    // Skip .DS_Store files
    if let Some(file_name) = path.file_name()
        && file_name == ".DS_Store"
    {
        return false;
    }

    // Skip dist and target directories, normally ignored by the watcher, but just in case
    if path
        .ancestors()
        .any(|p| p.ends_with("dist") || p.ends_with("target"))
    {
        return false;
    }

    true
}
