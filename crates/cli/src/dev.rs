use std::{
    io::{self},
    path::Path,
};

pub(crate) mod ipc;
pub(crate) mod server;

use maudit_ipc::{Message, MessageCommand};
use notify::{Event, RecursiveMode, Result, Watcher};
use std::sync::mpsc;

pub async fn coordinate_dev_env(cwd: String) -> io::Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(tx).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch(Path::new("./src"), RecursiveMode::Recursive)
        .unwrap();

    watcher
        .watch(Path::new("./content"), RecursiveMode::Recursive)
        .unwrap();

    let (build_process_writer, build_process_reader) = ipc::start_build_process(cwd);

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
                _ => {}
            }
        }
    });

    let web_server_thread = tokio::spawn(server::start_web_server());

    for res in rx {
        match res {
            Ok(event) => match event.kind {
                notify::EventKind::Create(_) => {
                    println!("created {:?}", event.paths);
                }
                notify::EventKind::Modify(_) => {
                    build_process_writer
                        .send(Message {
                            command: MessageCommand::Build,
                            data: None,
                        })
                        .unwrap();
                }
                notify::EventKind::Remove(_) => {
                    println!("removed {:?}", event.paths);
                }
                _ => {}
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    // Wait for the build process to finish
    web_server_thread.await.unwrap();
    let _ = build_process_thread.join();

    Ok(())
}
