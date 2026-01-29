pub(crate) mod server;

mod build;
mod dep_tracker;
mod filterer;

use notify::{
    EventKind, RecursiveMode,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, new_debouncer};
use quanta::Instant;
use server::WebSocketMessage;
use std::{fs, path::{Path, PathBuf}};
use tokio::{
    signal,
    sync::{broadcast, mpsc::channel},
    task::JoinHandle,
};
use tracing::{error, info};

use crate::dev::build::BuildManager;

pub async fn start_dev_env(cwd: &str, host: bool, port: Option<u16>) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    info!(name: "dev", "Preparing dev environment…");

    let (sender_websocket, _) = broadcast::channel::<WebSocketMessage>(100);

    // Create build manager (it will create its own status state internally)
    let build_manager = BuildManager::new(sender_websocket.clone());

    // Do initial build
    info!(name: "build", "Doing initial build…");
    let initial_build_success = build_manager.do_initial_build().await?;

    // Set up file watching with debouncer
    let (tx, mut rx) = channel::<DebounceEventResult>(1000);

    let directories = fs::read_dir(cwd)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !matches!(file_name, "target" | ".git" | "dist")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    let mut debouncer = new_debouncer(
        std::time::Duration::from_millis(100),
        None,
        move |result: DebounceEventResult| {
            tx.blocking_send(result).unwrap_or(());
        },
    )?;

    // Watch the root directly both for changes to files like Cargo.toml and for new directories
    debouncer.watch(cwd, RecursiveMode::NonRecursive)?;

    // It'd seems like it'd be much easier to just watch recursively from cwd, but the problem is that this ends up
    // watching a looooooot of files that we don't want to watch (target directory, dist directory, .git directory, etc.) which is causing about a million issues in Notify.
    // The fork of Notify we use has support for filtering while watching, but it doesn't seem to work super well in practice.
    // So instead we just watch the top-level directories (excluding known ones to ignore) and then add/remove watches for new/deleted directories as needed.
    for dir in &directories {
        debouncer.watch(dir, RecursiveMode::Recursive)?;
    }

    let mut web_server_thread: Option<tokio::task::JoinHandle<()>> = None;

    // If initial build succeeded, start web server immediately
    if initial_build_success {
        info!(name: "dev", "Starting web server...");
        web_server_thread = Some(tokio::spawn(server::start_dev_web_server(
            start_time,
            sender_websocket.clone(),
            host,
            port,
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
                            // TODO: Handle rescan events, I don't fully understand the implication of them yet
                            // some issues:
                            // - https://github.com/notify-rs/notify/issues/434
                            // - https://github.com/notify-rs/notify/issues/412

                            let should_rebuild = events.iter().any(should_rebuild_for_event);

                            // If new folder are created or removed, add/remove watches as needed
                            for event in &events {
                                if let EventKind::Create(CreateKind::Folder) = event.kind {
                                    for path in &event.paths {
                                        if should_watch_path(path) {
                                            if let Err(e) = debouncer.watch(path, RecursiveMode::Recursive) {
                                                error!(name: "watch", "Failed to add watch for new directory {:?}: {}", path, e);
                                            } else {
                                                info!(name: "watch", "Added watch for new directory {:?}", path);
                                            }
                                        }
                                    }
                                }

                                // TODO: This doesn't seem to always work, sometimes removed folders are considered renames (maybe because of trash?), but it's fine I think
                                if let EventKind::Remove(RemoveKind::Folder) = event.kind {
                                    for path in &event.paths {
                                        if should_watch_path(path) {
                                            if let Err(e) = debouncer.unwatch(path) {
                                                error!(name: "watch", "Failed to remove watch for directory {:?}: {}", path, e);
                                            } else {
                                                info!(name: "watch", "Removed watch for directory {:?}", path);
                                            }
                                        }
                                    }
                                }
                            }

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
                                                    port,
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
                                    // Normal rebuild - check if we need full recompilation or just rerun
                                    let changed_paths: Vec<PathBuf> = events.iter()
                                        .flat_map(|e| e.paths.iter().cloned())
                                        .collect();
                                    
                                    let needs_recompile = build_manager_watcher.needs_recompile(&changed_paths).await;
                                    
                                    if needs_recompile {
                                        // Need to recompile - spawn in background so file watcher can continue
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
                                    } else {
                                        // Just rerun the binary without recompiling
                                        info!(name: "watch", "Non-dependency files changed, rerunning binary...");
                                        let build_manager_clone = build_manager_watcher.clone();
                                        tokio::spawn(async move {
                                            match build_manager_clone.rerun_binary().await {
                                                Ok(_) => {
                                                    // Rerun completed (success or failure already logged)
                                                }
                                                Err(e) => {
                                                    error!(name: "build", "Failed to rerun binary: {}", e);
                                                }
                                            }
                                        });
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

    // Wait for either the web server, file watcher, or shutdown signal
    tokio::select! {
        _ = shutdown_signal() => {
            info!(name: "dev", "Shutting down dev environment...");
        }
        _ = async {
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
        } => {}
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
        .any(|p| p.ends_with("dist") || p.ends_with("target") || p.ends_with(".git"))
    {
        return false;
    }

    true
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
