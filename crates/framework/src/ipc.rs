use ipc_channel::ipc::{self, IpcReceiver, IpcSender};
use maudit_ipc::{Message, MessageCommand};

use crate::{content::ContentSources, execute_build, page::FullPage, BuildOptions, BuildOutput};

pub fn setup_ipc_server(
    server_name: &str,
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
    async_runtime: &tokio::runtime::Runtime,
) -> std::result::Result<BuildOutput, dyn_eq::Box<(dyn std::error::Error + 'static)>> {
    let (to_child, from_parent): (IpcSender<Message>, IpcReceiver<Message>) =
        ipc::channel().unwrap();
    let (to_parent, from_child): (IpcSender<Message>, IpcReceiver<Message>) =
        ipc::channel().unwrap();

    let bootstrap = IpcSender::connect(server_name.into()).unwrap();
    bootstrap.send((to_child, from_child)).unwrap();

    // Send ready message
    to_parent.send(Message {
        command: MessageCommand::Ready,
        data: None,
    })?;

    // Send initial build message
    to_parent.send(Message {
        command: MessageCommand::InitialBuild,
        data: None,
    })?;

    let mut initial_build = execute_build(routes, &mut content_sources, &options, async_runtime)?;

    // Send initial build finished message
    to_parent.send(Message {
        command: MessageCommand::InitialBuildFinished,
        data: None,
    })?;

    // Infinite loop for further messages
    loop {
        if let Ok(data) = from_parent.recv() {
            println!("Client received: {:?}", data);

            match data.command {
                MessageCommand::Build => {
                    initial_build =
                        execute_build(routes, &mut content_sources, &options, async_runtime)?;

                    to_parent.send(Message {
                        command: MessageCommand::BuildFinished,
                        data: None,
                    })?;
                }
                MessageCommand::Exit => {
                    println!("Client is exiting...");
                    break;
                }
                _ => {
                    println!("Client is still running...");
                }
            }
        }
    }

    Ok(initial_build)
}
