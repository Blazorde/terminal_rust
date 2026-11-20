use std::process::Command;
use std::{env, ffi::OsString};

use crate::utils::shell_env;

pub fn run(name: &str, executable: &str, args: &[String]) -> Result<(), String> {
    let program = bundled_program(executable).unwrap_or_else(|| OsString::from(executable));
    let mut command = Command::new(&program);
    shell_env::apply_to(&mut command);
    let status = command.args(args).status().map_err(|error| {
        format!(
            "{name}: cannot start {}: {error}",
            program.to_string_lossy()
        )
    })?;

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
