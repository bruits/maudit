use std::path::Path;
use std::process::Command;
use std::time::Instant;

use log::info;

/// Run the Tailwind CSS CLI on a given input file and return the processed CSS.
pub fn run_tailwind(
    tailwind_path: &Path,
    input_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let start_tailwind = Instant::now();
    let mut command = Command::new(tailwind_path);
    command.args(["--input", &input_path.to_string_lossy()]);

    if crate::is_dev() {
        command.arg("--map");
    } else {
        command.arg("--minify");
    }

    let tailwind_output = command.output()?;

    if !tailwind_output.status.success() {
        let stderr = String::from_utf8_lossy(&tailwind_output.stderr);
        return Err(format!(
            "Tailwind CSS process failed with status {}: {}",
            tailwind_output.status, stderr
        )
        .into());
    }

    info!("Tailwind took {:?}", start_tailwind.elapsed());

    Ok(String::from_utf8_lossy(&tailwind_output.stdout).into_owned())
}
