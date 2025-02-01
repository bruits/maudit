use std::io::{self};

pub(crate) mod server;

mod filterer;

use filterer::DevServerFilterer;
use server::WebSocketMessage;
use std::sync::Arc;
use tokio::sync::broadcast;
use watchexec::{
    command::{Command, Program},
    Watchexec,
};

pub async fn coordinate_dev_env(cwd: &str) -> io::Result<()> {
    let (sender_websocket, _) = broadcast::channel::<WebSocketMessage>(100);

    let web_server_thread: tokio::task::JoinHandle<()> =
        tokio::spawn(server::start_dev_web_server(sender_websocket.clone()));

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
                    job.start();
                    let ticket = job.to_wait();
                    ticket.await;

                    match browser_websocket.send(WebSocketMessage {
                        data: "done".into(),
                    }) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error sending message to browser: {:?}", e);
                        }
                    }
                }

                for event in action.events.iter() {
                    eprintln!("EVENT: {event:?}");
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
