use std::{
    io::{self},
    path::Path,
    time::Duration,
};

pub(crate) mod ipc;
pub(crate) mod server;

use event::ModifyKind;
use maudit_ipc::{Message, MessageCommand};
use notify_debouncer_full::{new_debouncer, notify::*};
use server::WebSocketMessage;
use tokio::sync::broadcast;

pub async fn coordinate_dev_env(cwd: String) -> io::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // no specific tickrate, max debounce time 2 seconds
    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer
        .watcher()
        .watch(Path::new("./src"), RecursiveMode::Recursive)
        .unwrap();

    debouncer
        .watcher()
        .watch(Path::new("./content"), RecursiveMode::Recursive)
        .unwrap();

    let (build_process_writer, build_process_reader) = ipc::start_build_process(cwd);

    let (tx_ws, _) = broadcast::channel::<WebSocketMessage>(100);

    let web_server_thread = tokio::spawn(server::start_web_server(tx_ws.clone()));

    let build_process_thread = std::thread::spawn(move || loop {
        if let Ok(data) = build_process_reader.recv() {
            println!("Server received: {:?}", data);

            match data.command {
                MessageCommand::InitialBuild => {
                    println!("Doing initial warming build...");
                }
                MessageCommand::InitialBuildFinished => {
                    println!("Done with initial build!");
                }
                MessageCommand::BuildFinished => {
                    println!("Done with build!");

                    // Send a message to the websocket that a build is done
                    match tx_ws.send(WebSocketMessage {
                        data: "done".to_string(),
                    }) {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e);
                            println!("Error sending message to websocket: {:?}", e.0);
                        }
                    }
                }
                _ => {}
            }
        }
    });

    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| match event.event.kind {
                EventKind::Create(_)
                | EventKind::Remove(_)
                | EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Name(_)) => {
                    build_process_writer
                        .send(Message {
                            command: MessageCommand::Build,
                            data: None,
                        })
                        .unwrap();
                }
                _ => {}
            }),
            Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
        }
        println!();
    }

    // Wait for the build process to finish
    web_server_thread.await.unwrap();
    let _ = build_process_thread.join();

    Ok(())
}
