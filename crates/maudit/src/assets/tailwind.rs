use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Instant,
};

use log::{debug, info};
use rolldown::{
    ModuleType,
    plugin::{HookUsage, Plugin},
};

/// Background Node sidecar process that handles Tailwind CSS compilation
#[derive(Debug)]
pub struct TailwindProcessor {
    sidecar_process: Arc<Mutex<Option<Child>>>,
    message_counter: Arc<Mutex<u64>>,
}

impl TailwindProcessor {
    /// Create a new TailwindProcessor that spawns the Node sidecar
    pub fn new(_tailwind_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let processor = Self {
            sidecar_process: Arc::new(Mutex::new(None)),
            message_counter: Arc::new(Mutex::new(0)),
        };
        processor.start()?;
        Ok(processor)
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut process_guard = self.sidecar_process.lock().unwrap();
        let mut child = self.spawn_sidecar_process()?;

        // Read and discard the ready signal
        let stdout = child.stdout.take().ok_or("Failed to get stdout handle")?;
        let mut reader = BufReader::new(stdout);

        // Read status line
        let mut status_line = String::new();
        reader.read_line(&mut status_line)?;

        if status_line.trim() != "READY" {
            return Err(format!("Expected READY, got: {}", status_line.trim()).into());
        }

        debug!("Sidecar ready signal received");

        // Put stdout back
        child.stdout = Some(reader.into_inner());

        *process_guard = Some(child);
        info!("Node sidecar process started successfully");
        Ok(())
    }

    fn spawn_sidecar_process(&self) -> Result<Child, Box<dyn std::error::Error>> {
        debug!("Spawning Node sidecar via npx maudit-node-sidecar");

        let child = Command::new("npx")
            .arg("maudit-node-sidecar")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start Node sidecar: {}", e))?;

        debug!("Node sidecar spawned with PID: {:?}", child.id());
        Ok(child)
    }

    pub fn process_css(&self, input_file: &str) -> Result<String, Box<dyn std::error::Error>> {
        info!(
            "TailwindProcessor::process_css() called for file: {}",
            input_file
        );

        let mut process_guard = self.sidecar_process.lock().unwrap();
        let child = process_guard
            .as_mut()
            .ok_or("Sidecar process not running")?;

        // Check if process is still alive
        if let Ok(Some(status)) = child.try_wait() {
            // Process has exited, try to get stderr
            let stderr = if let Some(mut stderr) = child.stderr.take() {
                let mut err = String::new();
                std::io::Read::read_to_string(&mut stderr, &mut err).ok();
                err
            } else {
                "No stderr available".to_string()
            };
            return Err(format!(
                "Sidecar process exited with status: {}. Stderr: {}",
                status, stderr
            )
            .into());
        }

        // Generate unique message ID
        let message_id = {
            let mut counter = self.message_counter.lock().unwrap();
            *counter += 1;
            format!("tailwind-{}", *counter)
        };

        // Create message manually
        let minify = !crate::is_dev();
        let message_json = format!(
            r#"{{"type":"tailwind","id":"{}","inputFile":"{}","minify":{}}}"#,
            message_id,
            input_file.replace('\\', "\\\\").replace('"', "\\\""),
            minify
        );

        // Send message to sidecar
        let stdin = child.stdin.as_mut().ok_or("Failed to get stdin handle")?;
        writeln!(stdin, "{}", message_json)?;
        stdin.flush()?;

        debug!("Sent message to sidecar: {}", message_json);

        // Read response from sidecar
        let stdout = child.stdout.as_mut().ok_or("Failed to get stdout handle")?;
        let mut reader = BufReader::new(stdout);

        // Read status line (OK or ERROR)
        let mut status_line = String::new();
        reader.read_line(&mut status_line)?;
        let status = status_line.trim();

        // Read length line
        let mut length_line = String::new();
        reader.read_line(&mut length_line)?;
        let length: usize = length_line
            .trim()
            .parse()
            .map_err(|e| format!("Failed to parse length '{}': {}", length_line.trim(), e))?;

        // Read data
        let mut buffer = vec![0u8; length];
        std::io::Read::read_exact(&mut reader, &mut buffer)?;

        // Skip trailing newline
        let mut newline = [0u8; 1];
        std::io::Read::read_exact(&mut reader, &mut newline)?;

        let content = String::from_utf8(buffer)?;

        match status {
            "OK" => {
                debug!(
                    "Received successful response from sidecar ({} bytes)",
                    length
                );
                Ok(content)
            }
            "ERROR" => Err(format!("Tailwind sidecar error: {}", content).into()),
            _ => Err(format!("Unknown status from sidecar: {}", status).into()),
        }
    }

    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut process_guard = self.sidecar_process.lock().unwrap();

        if let Some(mut process) = process_guard.take() {
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

/// Rolldown plugin to process select CSS files with the Tailwind CSS processor.
#[derive(Debug)]
pub struct TailwindPlugin {
    pub tailwind_entries: Vec<PathBuf>,
    pub processor: Arc<TailwindProcessor>,
}

impl TailwindPlugin {
    /// Create a new TailwindPlugin with a background processor
    pub fn new(tailwind_entries: Vec<PathBuf>, processor: Arc<TailwindProcessor>) -> Self {
        Self {
            tailwind_entries,
            processor,
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

            info!("Using Tailwind processor for {}", args.id);

            let output = self
                .processor
                .process_css(args.id)
                .unwrap_or_else(|e| panic!("Tailwind processor failed: {}", e));

            info!("Tailwind took {:?}", start_tailwind.elapsed());

            return Ok(Some(rolldown::plugin::HookTransformOutput {
                code: Some(output),
                ..Default::default()
            }));
        }

        Ok(None)
    }
}
