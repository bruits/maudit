use std::process::{Command, Stdio};

use tracing::{debug, error};

pub fn start_build() {
    match Command::new("cargo")
        .arg("run")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(mut child) => match child.wait() {
            Ok(exit_code) => {
                if exit_code.success() {
                    debug!(name: "build", "Build succeeded");
                } else {
                    error!(name: "build", "Build failed");
                }
            }
            Err(err) => {
                error!(name: "build", "Failed to build project: {:?}", err);
            }
        },
        Err(err) => {
            error!(name: "build", "Failed to spawn cargo: {:?}", err);
        }
    }
}
