use std::path::{Path, PathBuf};

use super::{CommandResult, ShellState, map_prefixed_error};
use crate::app::parser::{continuation_prefix, parse_input};
use crate::config;
use crate::render;

pub(super) fn run(args: &[String], state: &mut ShellState) -> CommandResult {
    let Some(action) = args.first().map(String::as_str) else {
        return CommandResult::Error("script: expected create, run, startup or list".into());
    };
    match action {
        "create" if args.len() == 2 => create(&args[1]),
        "run" if args.len() == 2 => run_file(&args[1], state),
        "startup" if args.len() == 2 => set_startup(&args[1], state),
        "list" if args.len() == 1 => list(),
        "create" | "run" | "startup" | "list" => {
            CommandResult::Error("script: invalid arguments; run 'help' for usage".into())
        }
        _ => CommandResult::Error(format!("script: unknown action '{action}'")),
    }
}

fn create(name: &str) -> CommandResult {
    if !crate::config::is_valid_user_command_name(name) {
        return CommandResult::Error(
            "script: name must contain lowercase letters, digits, '-' or '_'".into(),
        );
    }
    let path = PathBuf::from("scripts").join(format!("{name}.trs"));
    if let Err(error) = std::fs::create_dir_all("scripts") {
        return CommandResult::Error(format!("script: cannot create scripts directory: {error}"));
    }
    let content = format!(
        "# Terminal Rust script: {name}\n# One command per line. Lines ending in ` continue on the next line.\n\n"
    );
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return CommandResult::Error(format!("script: {} already exists", path.display()));
        }
        Err(error) => {
            return CommandResult::Error(format!(
                "script: cannot create {}: {error}",
                path.display()
            ));
        }
    };
    if let Err(error) = std::io::Write::write_all(&mut file, content.as_bytes()) {
        return CommandResult::Error(format!("script: cannot write {}: {error}", path.display()));
    }
    map_prefixed_error(
        "script",
        render::stdout(format!("Created {}.\n", path.display())),
    )
}

fn list() -> CommandResult {
    let entries = match std::fs::read_dir("scripts") {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return map_prefixed_error("script", render::stdout("No scripts found.\n"));
        }
        Err(error) => return CommandResult::Error(format!("script: cannot list scripts: {error}")),
    };
    let mut scripts = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                return CommandResult::Error(format!(
                    "script: cannot read scripts directory entry: {error}"
                ));
            }
        };
        let path = entry.path();
        if is_script_file(&path) {
            scripts.push(path);
        }
    }
    scripts.sort();
    let output = if scripts.is_empty() {
        "No scripts found.\n".to_string()
    } else {
        scripts
            .iter()
            .map(|path| format!("{}\n", path.display()))
            .collect()
    };
    map_prefixed_error("script", render::stdout(output))
}

fn run_file(path: &str, state: &mut ShellState) -> CommandResult {
    const MAX_SCRIPT_DEPTH: usize = 16;
    const MAX_SCRIPT_BYTES: u64 = 1_048_576;
    if state.script_depth >= MAX_SCRIPT_DEPTH {
        return CommandResult::Error("script: maximum nesting depth exceeded".into());
    }
    let script_path = PathBuf::from(path);
    let metadata = match std::fs::metadata(&script_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return CommandResult::Error(format!("script: {}: {error}", script_path.display()));
        }
    };
    if !metadata.is_file() || metadata.len() > MAX_SCRIPT_BYTES {
        return CommandResult::Error(format!(
            "script: {} must be a file no larger than 1 MiB",
            script_path.display()
        ));
    }
    if !is_script_file(&script_path) {
        return CommandResult::Error("script: expected a .trs file".into());
    }
    let content = match std::fs::read_to_string(&script_path) {
        Ok(content) => content,
        Err(error) => {
            return CommandResult::Error(format!(
                "script: cannot read {}: {error}",
                script_path.display()
            ));
        }
    };

    state.script_depth += 1;
    let result = execute_contents(&content, &script_path, state);
    state.script_depth -= 1;
    result
}

fn execute_contents(content: &str, script_path: &Path, state: &mut ShellState) -> CommandResult {
    let mut pending = String::new();
    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(prefix) = continuation_prefix(line) {
            pending.push_str(prefix);
            pending.push(' ');
            continue;
        }
        pending.push_str(line);
        let result = match parse_input(&pending) {
            Ok(args) if args.is_empty() => CommandResult::Continue,
            Ok(args) => super::execute(&args, state),
            Err(error) => CommandResult::Error(error),
        };
        pending.clear();
        match result {
            CommandResult::Continue => {}
            CommandResult::Exit => return CommandResult::Exit,
            CommandResult::Error(error) => {
                return CommandResult::Error(format!(
                    "script: {}:{line_number}: {error}",
                    script_path.display()
                ));
            }
        }
    }
    if !pending.is_empty() {
        return CommandResult::Error(format!(
            "script: {} ends after a continuation marker",
            script_path.display()
        ));
    }
    CommandResult::Continue
}

fn set_startup(path: &str, state: &mut ShellState) -> CommandResult {
    let startup_script = if path == "off" {
        None
    } else {
        let script_path = match std::fs::canonicalize(path) {
            Ok(path) => path,
            Err(error) => {
                return CommandResult::Error(format!("script: cannot use {path}: {error}"));
            }
        };
        if !script_path.is_file() || !is_script_file(&script_path) {
            return CommandResult::Error(
                "script: startup target must be an existing .trs file".into(),
            );
        }
        Some(script_path.to_string_lossy().into_owned())
    };
    if let Err(error) = config::save_startup_script(startup_script.clone()) {
        return CommandResult::Error(format!("script: {error}"));
    }
    state.startup_script = startup_script;
    let message = match state.startup_script.as_deref() {
        Some(path) => format!("Startup script set to {path}.\n"),
        None => "Startup script disabled.\n".into(),
    };
    map_prefixed_error("script", render::stdout(message))
}

fn is_script_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("trs"))
}
