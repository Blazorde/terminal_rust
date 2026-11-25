use std::path::{Path, PathBuf};

use super::{CommandResult, ShellState, map_prefixed_error};
use crate::app::parser::{continuation_prefix, split_chain};
use crate::config;
use crate::render;

pub(crate) const MAX_SCRIPT_BYTES: u64 = 1_048_576;

pub(super) fn run(args: &[String], state: &mut ShellState) -> CommandResult {
    let Some(action) = args.first().map(String::as_str) else {
        return CommandResult::Error("script: expected create, run, startup or list".into());
    };
    match action {
        "create" if args.len() == 2 => create(&args[1]),
        "run" if args.len() == 2 => run_file("script", &args[1], state, true),
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

/// Run a file in the current shell. `command` names the command in messages, and
/// `scripts_only` restricts it to `.trs` files (`script run`); `source` takes any
/// file the user names.
pub(super) fn run_file(
    command: &str,
    path: &str,
    state: &mut ShellState,
    scripts_only: bool,
) -> CommandResult {
    const MAX_SCRIPT_DEPTH: usize = 16;
    if state.script_depth >= MAX_SCRIPT_DEPTH {
        return CommandResult::Error(format!("{command}: maximum nesting depth exceeded"));
    }
    let script_path = PathBuf::from(path);
    let metadata = match std::fs::metadata(&script_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return CommandResult::Error(format!("{command}: {}: {error}", script_path.display()));
        }
    };
    if !metadata.is_file() || metadata.len() > MAX_SCRIPT_BYTES {
        return CommandResult::Error(format!(
            "{command}: {} must be a file no larger than 1 MiB",
            script_path.display()
        ));
    }
    if scripts_only && !is_script_file(&script_path) {
        return CommandResult::Error(format!("{command}: expected a .trs file"));
    }
    let content = match std::fs::read_to_string(&script_path) {
        Ok(content) => content,
        Err(error) => {
            return CommandResult::Error(format!(
                "{command}: cannot read {}: {error}",
                script_path.display()
            ));
        }
    };

    state.script_depth += 1;
    let result = execute_contents(command, &content, &script_path, state);
    state.script_depth -= 1;
    result
}

fn execute_contents(
    command: &str,
    content: &str,
    script_path: &Path,
    state: &mut ShellState,
) -> CommandResult {
    let mut assembler = CommandAssembler::default();
    for (index, line) in content.lines().enumerate() {
        let Some(complete) = assembler.push_line(line) else {
            continue;
        };
        match super::run_line(&complete, state) {
            CommandResult::Continue => {}
            CommandResult::Exit => return CommandResult::Exit,
            CommandResult::Error(error) => {
                return CommandResult::Error(format!(
                    "{command}: {}:{}: {error}",
                    script_path.display(),
                    index + 1
                ));
            }
        }
    }
    if assembler.is_unfinished() {
        return CommandResult::Error(format!(
            "{command}: {} ends inside an unfinished command",
            script_path.display()
        ));
    }
    CommandResult::Continue
}

/// Groups the lines of a script into complete commands: comments and blank lines are
/// left out, continued lines are joined, and a block or a here-document is kept until
/// it is closed.
#[derive(Default)]
pub(crate) struct CommandAssembler {
    pending: String,
    started: bool,
}

impl CommandAssembler {
    /// Add a line, without its line ending, and give back the command it completes.
    pub(crate) fn push_line(&mut self, raw_line: &str) -> Option<String> {
        // Some editors, and PowerShell pipes, start the text with a byte order mark.
        let raw_line = if std::mem::replace(&mut self.started, true) {
            raw_line
        } else {
            raw_line.strip_prefix('\u{feff}').unwrap_or(raw_line)
        };
        if self.pending.ends_with('\n') && super::heredoc_is_open(&self.pending) {
            // The body of a here-document is kept as it is: blanks, `#` and all.
            self.pending.push_str(raw_line);
        } else {
            let line = raw_line.trim();
            if line.starts_with('#') || line.is_empty() {
                return None;
            }
            if let Some(prefix) = continuation_prefix(line) {
                self.pending.push_str(prefix);
                self.pending.push(' ');
                return None;
            }
            self.pending.push_str(line);
        }
        // A block or a here-document that is not closed goes on over the next line.
        if super::input_is_incomplete(&self.pending) {
            self.pending.push('\n');
            return None;
        }
        Some(std::mem::take(&mut self.pending))
    }

    /// Whether lines are still waiting for the end of their command.
    pub(crate) fn is_unfinished(&self) -> bool {
        !self.pending.is_empty()
    }
}

/// Whether the complete command `input` can be parsed; nothing runs.
pub(crate) fn check_syntax(input: &str) -> Result<(), String> {
    let rewritten = super::heredoc::rewrite(input)?;
    super::flow::parse(split_chain(&rewritten)?).map(drop)
}

impl ShellState {
    /// `$0` and `$1`, `$2`, ... of a script or of `-c`.
    pub(crate) fn set_arguments(&mut self, name: Option<&str>, arguments: &[String]) {
        self.script_name = name.map(str::to_string);
        self.frames = vec![super::functions::Frame::for_file(arguments)];
    }

    /// What runs comes from a script, not from someone typing: as in a sourced file,
    /// `alias` does not confirm.
    pub(crate) fn enter_script(&mut self) {
        self.script_depth += 1;
    }

    /// The end of what [`ShellState::enter_script`] began.
    pub(crate) fn leave_script(&mut self) {
        self.script_depth = self.script_depth.saturating_sub(1);
    }
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
