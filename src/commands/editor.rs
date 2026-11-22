use std::process::Command;
use std::{env, ffi::OsString};

use crate::utils::job_control::{self, Finished};
use crate::utils::shell_env;

pub fn run(name: &str, executable: &str, args: &[String]) -> Result<(), String> {
    let program = bundled_program(executable).unwrap_or_else(|| OsString::from(executable));
    let mut command = Command::new(&program);
    shell_env::prepare_child(&mut command);
    command.args(args);
    let text = format!("{executable} {}", args.join(" "));
    let status = match job_control::run_foreground(command, text.trim()).map_err(|error| {
        format!(
            "{name}: cannot start {}: {error}",
            program.to_string_lossy()
        )
    })? {
        Finished::Exited(status) => status,
        Finished::Stopped => return Ok(()),
    };

    if status.success() {
        Ok(())
    } else {
        Err(format!("{name}: editor exited with status {status}"))
    }
}

fn bundled_program(executable: &str) -> Option<OsString> {
    let variable = match executable {
        "nano" => "TERMINAL_RUST_NANO",
        "nvim" => "TERMINAL_RUST_NVIM",
        _ => return None,
    };
    env::var_os(variable).filter(|value| !value.is_empty())
}
