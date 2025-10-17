use std::{
    fs::read,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    time::Instant,
};

use log::info;
use rolldown::{
    ModuleType,
    plugin::{HookUsage, Plugin},
};

/// Rolldown plugin to process select CSS files with the Tailwind CSS CLI.
#[derive(Debug)]
pub struct TailwindPlugin {
    pub tailwind_path: PathBuf,
    pub tailwind_entries: Vec<PathBuf>,
}

impl Plugin for TailwindPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "builtin:tailwind".into()
    }

    fn register_hook_usage(&self) -> rolldown::plugin::HookUsage {
        HookUsage::Transform
    }

    async fn transform(
        &self,
        _ctx: rolldown::plugin::SharedTransformPluginContext,
        args: &rolldown::plugin::HookTransformArgs<'_>,
    ) -> rolldown::plugin::HookTransformReturn {
        if *args.module_type != ModuleType::Css {
            return Ok(None);
        }

        if self
            .tailwind_entries
            .iter()
            .any(|entry| entry.canonicalize().unwrap().to_string_lossy() == args.id)
        {
            let content = read(args.id).unwrap_or_else(|e| {
                panic!(
                    "Failed to read Tailwind CSS input file '{}': {}",
                    &args.id, e
                )
            });

            let start_tailwind = Instant::now();
            let mut command = Command::new(&self.tailwind_path);
            command.args(["--input", "-", "--output", "-"]);

            // Add minify in production, source maps in development
            if !crate::is_dev() {
                command.arg("--minify");
            }

            let tailwind_command = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = tailwind_command
                .spawn()
                .expect("Failed to spawn Tailwind CSS process");

            {
                let stdin = child
                    .stdin
                    .as_mut()
                    .expect("Failed to open stdin for Tailwind CSS");
                stdin
                    .write_all(&content)
                    .expect("Failed to write to Tailwind CSS stdin");
            }

            let output = child
                .wait_with_output()
                .expect("Failed to read Tailwind CSS process output");

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let error_message = format!(
                    "Tailwind CSS process failed with status {}: {}",
                    output.status, stderr
                );
                panic!("{}", error_message);
            }

            info!("Tailwind took {:?}", start_tailwind.elapsed());

            let output = String::from_utf8_lossy(&output.stdout);

            return Ok(Some(rolldown::plugin::HookTransformOutput {
                code: Some(output.into_owned()),
                ..Default::default()
            }));
        }

        Ok(None)
    }
}
