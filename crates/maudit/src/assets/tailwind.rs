use std::{io::Error, path::PathBuf, process::Command, time::Instant};

use log::info;
use oxc_sourcemap::SourceMap;
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
            let start_tailwind = Instant::now();
            let mut command = Command::new(&self.tailwind_path);
            command.args(["--input", args.id]);

            // Add minify in production, source maps in development
            if !crate::is_dev() {
                command.arg("--minify");
            }
            if crate::is_dev() {
                command.arg("--map");
            }

            let tailwind_output = command.output()?;

            if !tailwind_output.status.success() {
                let stderr = String::from_utf8_lossy(&tailwind_output.stderr);
                let error_message = format!(
                    "Tailwind CSS process failed with status {}: {}",
                    tailwind_output.status, stderr
                );

                return Err(Error::other(error_message).into());
            }

            info!("Tailwind took {:?}", start_tailwind.elapsed());

            let output = String::from_utf8_lossy(&tailwind_output.stdout);
            let (code, map) = if let Some((code, map)) = output.split_once("/*# sourceMappingURL") {
                (code.to_string(), Some(map.to_string()))
            } else {
                (output.to_string(), None)
            };

            if let Some(map) = map {
                let source_map = SourceMap::from_json_string(&map).ok();

                return Ok(Some(rolldown::plugin::HookTransformOutput {
                    code: Some(code),
                    map: source_map,
                    ..Default::default()
                }));
            }

            return Ok(Some(rolldown::plugin::HookTransformOutput {
                code: Some(code),
                ..Default::default()
            }));
        }

        Ok(None)
    }
}
