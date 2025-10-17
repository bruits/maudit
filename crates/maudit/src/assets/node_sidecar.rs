use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use log::{debug, info};

/// Background Node sidecar process that handles various JavaScript/Node operations
#[derive(Debug)]
pub struct NodeSidecar {
    process: Arc<Mutex<Option<Child>>>,
    message_counter: Arc<Mutex<u64>>,
    ready: Arc<Mutex<bool>>,
    binary_path: PathBuf,
}

impl NodeSidecar {
    /// Create a new NodeSidecar and spawn the process in the background
    pub fn new(node_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let sidecar = Self {
            process: Arc::new(Mutex::new(None)),
            message_counter: Arc::new(Mutex::new(0)),
            ready: Arc::new(Mutex::new(false)),
            binary_path: node_path,
        };
        sidecar.spawn_process_async()?;
        Ok(sidecar)
    }

    fn spawn_process_async(&self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Spawning Node sidecar process...");

        let child = Command::new(&self.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start Node sidecar: {}", e))?;

        debug!("Node sidecar spawned with PID: {:?}", child.id());

        let mut process_guard = self.process.lock().unwrap();
        *process_guard = Some(child);

        Ok(())
    }

    fn ensure_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Check if already ready
        {
            let ready_guard = self.ready.lock().unwrap();
            if *ready_guard {
                return Ok(());
            }
        }

        // Wait for ready signal
        let mut process_guard = self.process.lock().unwrap();
        let child = process_guard
            .as_mut()
            .ok_or("Sidecar process not running")?;

        // Read and discard the ready signal
        let stdout = child.stdout.take().ok_or("Failed to get stdout handle")?;
        let mut reader = BufReader::new(stdout);

        // Read status line
        let mut status_line = String::new();
        reader.read_line(&mut status_line)?;

        if status_line.trim() != "READY" {
            return Err(format!("Expected READY, got: {}", status_line.trim()).into());
        }

        debug!("Node sidecar ready signal received");

        // Put stdout back
        child.stdout = Some(reader.into_inner());

        // Mark as ready
        let mut ready_guard = self.ready.lock().unwrap();
        *ready_guard = true;

        info!("Node sidecar process ready");
        Ok(())
    }

    /// Process CSS with Tailwind
    pub fn process_tailwind(
        &self,
        input_file: &str,
        minify: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Ensure sidecar is ready before processing
        self.ensure_ready()?;

        info!(
            "NodeSidecar::process_tailwind() called for file: {}",
            input_file
        );

        let mut process_guard = self.process.lock().unwrap();
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
        let mut process_guard = self.process.lock().unwrap();

        if let Some(mut process) = process_guard.take() {
            info!("Stopping Node sidecar process");
            process.kill()?;
            process.wait()?;
        }

        Ok(())
    }
}

impl Drop for NodeSidecar {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            info!("Failed to stop Node sidecar process during drop: {}", e);
        }
    }
}
