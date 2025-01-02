use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    pub command: MessageCommand,
    pub data: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum MessageCommand {
    Ready,
    Exit,
    InitialBuild,
    InitialBuildFinished,
    Build,
    BuildFinished,
}

// TODO: This should probably use separate messages for the server / client, ha
