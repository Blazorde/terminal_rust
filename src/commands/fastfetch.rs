use std::process::Command;

use crate::utils::job_control::{self, Finished};
use crate::utils::shell_env;

pub fn run(args: &[String]) -> Result<(), String> {
    let mut command = Command::new("fastfetch");
    shell_env::prepare_child(&mut command);
    command.args(args);
    let text = format!("fastfetch {}", args.join(" "));
    let status = match job_control::run_foreground(command, text.trim()).map_err(|error| {
        format!(
            "cannot start 'fastfetch': {error}. Install Fastfetch and ensure it is available in PATH"
        )
    })? {
        Finished::Exited(status) => status,
        Finished::Stopped => return Ok(()),
    };
    if status.success() {
        Ok(())
    } else {
        Err(format!("Fastfetch exited with status {status}"))
    }
}
