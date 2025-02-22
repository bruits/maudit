use std::io::{self};

pub(crate) mod server;

mod filterer;

use filterer::DevServerFilterer;
use server::WebSocketMessage;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info};
use watchexec::{
    command::{Command, Program},
    job::CommandState,
    Watchexec,
};

use crate::logging::format_elapsed_time;

pub async fn start_dev_env(cwd: &str, host: bool) -> io::Result<()> {
    info!(name: "dev", "Preparing dev environment…");
    let (sender_websocket, _) = broadcast::channel::<WebSocketMessage>(100);

    let web_server_thread: tokio::task::JoinHandle<()> =
        tokio::spawn(server::start_dev_web_server(sender_websocket.clone(), host));

    let wx = Watchexec::new_async(move |mut action| {
        Box::new({
            let browser_websocket = sender_websocket.clone();

            async move {
                if action.signals().next().is_some() {
                    eprintln!("[Quitting...]");
                    action.quit();
                } else {
                    let (_, job) = action.create_job(Arc::new(Command {
                        program: Program::Exec {
                            prog: "cargo".into(),
                            args: vec![
                                "run".into(),
                                "--quiet".into(),
                                "--".into(),
                                "--quiet".into(),
                            ],
                        },
                        options: Default::default(),
                    }));
                    job.set_error_handler(|err| {
                        eprintln!("Error: {:?}", err);
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
                            format_elapsed_time(Ok(duration), &Default::default()).unwrap();

                        match status {
                            watchexec_events::ProcessEnd::ExitError(_) => {
                                error!(name: "build", "Rebuild failed with errors {}", formatted_elasped_time);
                            },
                            watchexec_events::ProcessEnd::Success => {
                                info!(name: "build", "Rebuild finished {}", formatted_elasped_time);
                            },
                            // TODO: Log the other statuses
                            _ => {}
                        }

                        match browser_websocket.send(WebSocketMessage {
                            data: "done".into(),
                        }) {
                            Ok(_) => {}
                            Err(e) => {
                                debug!("Error sending message to browser: {:?}", e);
                            }
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
