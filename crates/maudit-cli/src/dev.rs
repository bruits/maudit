use std::io::{self};

pub(crate) mod server;

mod filterer;

use colored::Colorize;
use filterer::DevServerFilterer;
use quanta::Instant;
use server::{update_status, WebSocketMessage};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info};
use watchexec::{
    command::{Command, Program, Shell},
    job::CommandState,
    Watchexec,
};

use crate::logging::{format_elapsed_time, FormatElapsedTimeOptions};

pub async fn start_dev_env(cwd: &str, host: bool) -> io::Result<()> {
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

    let wx = Watchexec::new_async(move |mut action| {
        Box::new({
            let browser_websocket = sender_websocket.clone();
            let current_status = current_status.clone();

            async move {
                if action.signals().next().is_some() {
                    action.quit();
                    return action;
                } else {
                    info!(name: "build", "Detected changes. Rebuilding…");

                    // TODO: This kinda sucks but watchexec doesn't support setting env vars on commands
                    // Maybe I need to use something else than watchexec
                    let (shell, command) = if cfg!(windows) {
                        ("cmd", "/C set MAUDIT_DEV=true && set MAUDIT_QUIET=true && cargo run --quiet")
                    } else {
                        ("sh", "MAUDIT_DEV=true MAUDIT_QUIET=true cargo run --quiet")
                    };

                    let (_, job) = action.create_job(Arc::new(Command {
                        program: Program::Shell {
                            shell: Shell::new(shell),
                            command: command.into(),
                            args: vec![],
                        },
                        options: Default::default(),
                    }));

                    job.set_error_handler(|err| {
                        eprintln!("Error: {:?}", err);
                    });
                    job.set_spawn_hook(|pre_spawn, _| {
                        let command: &mut tokio::process::Command = pre_spawn.command_mut();

                        command.stdout(std::process::Stdio::inherit()); // Show stdout in real-time with colors
                        command.stderr(std::process::Stdio::piped()); // Capture stderr for WebSocket
                    });
                    job.start();
                    job.to_wait().await;

                    // TODO: Find a way to extract the stdout and stderr from the job and show it to the user other than
                    // cargo logging

                    job.run(move |context| {
                        let CommandState::Finished {
                            status,
                            started,
                            finished,
                        } = context.current
                        else {
                            return;
                        };

                        let duration = *finished - *started;
                        let formatted_elasped_time =
                            format_elapsed_time(duration, &FormatElapsedTimeOptions::default_dev());

                        match status {
                            watchexec_events::ProcessEnd::ExitError(_) => {
                                error!(name: "build", "Rebuild failed with errors {}", formatted_elasped_time);

                                // Update status and send error message to browser
                                let websocket = browser_websocket.clone();
                                let status = current_status.clone();
                                tokio::spawn(async move {
                                    update_status(&websocket, status, "error", "Build failed with errors").await;
                                });
                            },
                            watchexec_events::ProcessEnd::Success => {
                                info!(name: "build", "Rebuild finished {}", formatted_elasped_time);

                                // Update status and send success message to browser
                                let websocket = browser_websocket.clone();
                                let status = current_status.clone();
                                tokio::spawn(async move {
                                    update_status(&websocket, status, "success", "Build completed successfully").await;
                                });
                            },
                            // TODO: Log the other statuses
                            _ => {}
                        }
                    });
                }

                for event in action.events.iter() {
                    debug!("EVENT: {event:?}");
                }

                action
            }
        })
    })
    .unwrap();

    wx.config.pathset([cwd]);
    wx.config.filterer(DevServerFilterer);

    let _ = wx.main().await;

    // Wait for the build process to finish
    web_server_thread.await.unwrap();

    Ok(())
}
