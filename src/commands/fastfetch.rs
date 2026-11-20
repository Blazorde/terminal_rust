use std::process::Command;

use crate::utils::shell_env;

pub fn run(args: &[String]) -> Result<(), String> {
    let mut command = Command::new("fastfetch");
    shell_env::apply_to(&mut command);
    let status = command
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
