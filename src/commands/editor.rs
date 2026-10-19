use std::process::Command;

pub fn run(name: &str, executable: &str, args: &[String]) -> Result<(), String> {
    let status = Command::new(executable)
        .args(args)
        .status()
        .map_err(|error| format!("{name}: cannot start {executable}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{name}: editor exited with status {status}"))
    }
}
