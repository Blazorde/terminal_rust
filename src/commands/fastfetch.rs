use std::process::Command;

pub fn run(args: &[String]) -> Result<(), String> {
    let status = Command::new("fastfetch")
        .args(args)
        .status()
        .map_err(|error| {
            format!(
                "cannot start 'fastfetch': {error}. Install Fastfetch and ensure it is available in PATH"
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Fastfetch exited with status {status}"))
    }
}
