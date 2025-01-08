use std::process::Command;

use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use maudit_ipc::{Message, MessageCommand};

type Bootstrap = (IpcSender<Message>, IpcReceiver<Message>);

pub fn start_build_process(cwd: &str) -> (IpcSender<Message>, IpcReceiver<Message>) {
    let (server0, server_name0) = IpcOneShotServer::<Bootstrap>::new().unwrap();

    // TODO: Handle errors
    let _ = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(["--ipc-server", &server_name0])
        .current_dir(cwd)
        .spawn();

    let (_receiver, (sender, receiver)): (IpcReceiver<Bootstrap>, Bootstrap) =
        server0.accept().unwrap();

    // Wait for ready message
    let message = receiver.recv().unwrap();

    if let MessageCommand::Ready = message.command {
        println!("Server is ready!");
    } else {
        // TODO: Anything else than ready is an error, but we should handle it better
        panic!("Server failed to start!");
    }

    (sender, receiver)
}
