use std::{
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Instant,
};

use log::info;
use oxc_sourcemap::SourceMap;
use rolldown::{
    ModuleType,
    plugin::{HookUsage, Plugin},
};

/// Background Tailwind CSS processor that maintains a warm process
/// for faster CSS compilation via stdin/stdout communication.
#[derive(Debug)]
pub struct TailwindProcessor {
    tailwind_path: PathBuf,
    warm_process: Arc<Mutex<Option<Child>>>,
}

impl TailwindProcessor {
    /// Create a new TailwindProcessor with the given binary path
    pub fn new(tailwind_path: PathBuf) -> Self {
        Self {
            tailwind_path,
            warm_process: Arc::new(Mutex::new(None)),
        }
    }

    /// Pre-warm a Tailwind process for faster first use
    pub fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("TailwindProcessor::start() called - pre-warming process");
        self.spawn_warm_process()
    }

    /// Spawn a new warm process
    fn spawn_warm_process(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut warm_guard = self.warm_process.lock().unwrap();

        info!(
            "Creating warm Tailwind command with binary: {}",
            self.tailwind_path.display()
        );
        let mut command = Command::new(&self.tailwind_path);
        command
            .args(["--input", "-", "--output", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add development/production flags
        if !crate::is_dev() {
            info!("Adding --minify flag for production");
            command.arg("--minify");
        }

        info!("Spawning warm Tailwind process...");
        let child = command
            .spawn()
            .map_err(|e| format!("Failed to start warm Tailwind process: {}", e))?;

        info!(
            "Warm Tailwind process spawned successfully with PID: {:?}",
            child.id()
        );
        *warm_guard = Some(child);
        Ok(())
    }

    /// Process CSS input using warm process if available, otherwise spawn fresh
    pub fn process_css(
        &self,
        input_css: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "TailwindProcessor::process_css() called with {} bytes of CSS",
            input_css.len()
        );

        // Try to use the warm process first
        let warm_process = {
            let mut warm_guard = self.warm_process.lock().unwrap();
            warm_guard.take()
        };

        let mut child = if let Some(process) = warm_process {
            info!("Using pre-warmed Tailwind process");
            process
        } else {
            info!("No warm process available, spawning fresh one");
            self.spawn_fresh_process()?
        };

        info!("Getting stdin handle...");
        let stdin = child.stdin.take().ok_or("Failed to get stdin handle")?;

        info!("Writing CSS to stdin...");
        {
            let mut stdin = stdin;
            stdin.write_all(input_css.as_bytes())?;
            info!("Stdin written and closed");
            // stdin gets dropped here, closing it
        }

        info!("Waiting for process to complete and reading output...");
        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            info!("Process failed with stderr: {}", stderr);
            return Err(format!("Tailwind process failed: {}", stderr).into());
        }

        let result = String::from_utf8_lossy(&output.stdout).to_string();
        info!(
            "Process completed successfully, got {} bytes of output",
            result.len()
        );

        Ok(result.trim().to_string())
    }

    /// Spawn a fresh process immediately
    fn spawn_fresh_process(&self) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Creating fresh Tailwind command with binary: {}",
            self.tailwind_path.display()
        );
        let mut command = Command::new(&self.tailwind_path);
        command
            .args(["--input", "-", "--output", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add development/production flags
        if !crate::is_dev() {
            command.arg("--minify");
        }

        info!("Spawning fresh Tailwind process...");
        let child = command
            .spawn()
            .map_err(|e| format!("Failed to start fresh Tailwind process: {}", e))?;

        info!(
            "Fresh Tailwind process spawned successfully with PID: {:?}",
            child.id()
        );
        Ok(child)
    }

    /// Stop any warm process
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut warm_guard = self.warm_process.lock().unwrap();

        if let Some(mut process) = warm_guard.take() {
            info!("Stopping warm Tailwind process");
            process.kill()?;
            process.wait()?;
        }

        Ok(())
    }
}

impl Drop for TailwindProcessor {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            info!("Failed to stop Tailwind process during drop: {}", e);
        }
    }
}

/// Rolldown plugin to process select CSS files with the Tailwind CSS CLI.
#[derive(Debug)]
pub struct TailwindPlugin {
    pub tailwind_path: PathBuf,
    pub tailwind_entries: Vec<PathBuf>,
    pub processor: Option<Arc<TailwindProcessor>>,
}

impl TailwindPlugin {
    /// Create a new TailwindPlugin with a background processor
    pub fn with_processor(
        tailwind_path: PathBuf,
        tailwind_entries: Vec<PathBuf>,
        processor: Arc<TailwindProcessor>,
    ) -> Self {
        Self {
            tailwind_path,
            tailwind_entries,
            processor: Some(processor),
        }
    }
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

            let output = if let Some(processor) = &self.processor {
                info!("Using background Tailwind processor for {}", args.id);
                // Use background processor
                let input_css = std::fs::read_to_string(args.id)
                    .unwrap_or_else(|e| panic!("Failed to read input file: {}", e));

                info!("Read {} bytes from input file", input_css.len());
                processor
                    .process_css(&input_css)
                    .unwrap_or_else(|e| panic!("Background Tailwind processor failed: {}", e))
            } else {
                // Use CLI mode
                let mut command = Command::new(&self.tailwind_path);
                command.args(["--input", args.id]);

                // Add minify in production, source maps in development
                if !crate::is_dev() {
                    command.arg("--minify");
                }
                if crate::is_dev() {
                    command.arg("--map");
                }

                let tailwind_output = command.output()
                    .unwrap_or_else(|e| {
                        let args_str = if crate::is_dev() {
                            format!("['--input', '{}', '--map']", args.id)
                        } else {
                            format!("['--input', '{}', '--minify']", args.id)
                        };
                        panic!(
                            "Failed to execute Tailwind CSS command, is it installed and is the path to its binary correct?\nCommand: '{}', Args: {}. Error: {}",
                            self.tailwind_path.display(),
                            args_str,
                            e
                        )
                    });

                if !tailwind_output.status.success() {
                    let stderr = String::from_utf8_lossy(&tailwind_output.stderr);
                    panic!(
                        "Tailwind CSS process failed with status {}: {}",
                        tailwind_output.status, stderr
                    );
                }

                String::from_utf8_lossy(&tailwind_output.stdout).to_string()
            };

            info!("Tailwind took {:?}", start_tailwind.elapsed());

            let (code, map) = if let Some((code, map)) = output.split_once("/*# sourceMappingURL") {
                (code.to_string(), Some(map.to_string()))
            } else {
                (output, None)
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
