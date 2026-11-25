//! Running without a prompt: the string of `-c`, a script file, or standard input
//! that is not a terminal. Commands run one at a time and their output is shown as
//! they end; a command that fails does not stop the ones that follow.

use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::ExitCode;

use crate::app::command::{CommandAssembler, MAX_SCRIPT_BYTES, check_syntax};
use crate::app::runner::{emit_result, load_config, load_engine};
use crate::core::{ExecutionStatus, ShellEngine};
use crate::render;
use crate::render::printable::printable;

// The statuses POSIX shells use for these failures.
const INPUT_ERROR: u8 = 1;
const SYNTAX_ERROR: u8 = 2;
const CANNOT_RUN: u8 = 126;
const NOT_FOUND: u8 = 127;

pub(super) fn run_command_string(command: &str, name: Option<&str>, args: &[String]) -> ExitCode {
    let lines = command.lines().map(|line| Ok(line.to_string()));
    run(batch_engine(name, args), lines, None)
}

pub(super) fn run_script(path: &str, args: &[String]) -> ExitCode {
    match read_script(path) {
        Ok(content) => {
            let lines = content.lines().map(|line| Ok(line.to_string()));
            run(batch_engine(Some(path), args), lines, Some(path))
        }
        Err(error) => {
            let _ = render::stderr_line(format_args!(
                "terminal_rust: {}: {}",
                printable(path),
                error.reason
            ));
            ExitCode::from(error.status)
        }
    }
}

pub(super) fn run_stdin() -> ExitCode {
    run(
        batch_engine(None, &[]),
        std::iter::from_fn(read_stdin_line),
        None,
    )
}

fn batch_engine(name: Option<&str>, args: &[String]) -> ShellEngine {
    let mut engine = load_engine(&load_config());
    engine.set_arguments(name, args);
    engine.enter_script();
    engine
}

/// Stdin is locked for one line only: `read`, `rm` and the programs that a command
/// starts take their own input from it.
fn read_stdin_line() -> Option<io::Result<String>> {
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(Ok(line.trim_end_matches(['\r', '\n']).to_string())),
        Err(error) => Some(Err(error)),
    }
}

/// Run the commands of `lines`, then the `EXIT` trap. `script` is the file named in
/// the error messages.
fn run(
    mut engine: ShellEngine,
    lines: impl Iterator<Item = io::Result<String>>,
    script: Option<&str>,
) -> ExitCode {
    let failure = run_commands(&mut engine, lines, script);
    let status = failure.unwrap_or_else(|| engine.exit_code());
    let trap = engine.finish();
    emit_result(&trap);
    // The trap changes the status of the shell only when it calls `exit`.
    ExitCode::from(if trap.status == ExecutionStatus::Exit {
        engine.exit_code()
    } else {
        status
    })
}

/// `Some` is the status of an input that could not be run to its end.
fn run_commands(
    engine: &mut ShellEngine,
    lines: impl Iterator<Item = io::Result<String>>,
    script: Option<&str>,
) -> Option<u8> {
    let mut assembler = CommandAssembler::default();
    let mut line_number = 0;
    for line in lines {
        line_number += 1;
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                report(
                    script,
                    line_number,
                    &format!("cannot read the input: {error}"),
                );
                return Some(INPUT_ERROR);
            }
        };
        let Some(command) = assembler.push_line(&line) else {
            continue;
        };
        // As in POSIX shells, nothing runs after text that cannot be understood.
        if let Err(error) = check_syntax(&command) {
            report(script, line_number, &error);
            return Some(SYNTAX_ERROR);
        }
        let mut result = engine.execute_line(&command);
        if let (Some(path), Some(error)) = (script, result.error.as_mut()) {
            *error = located(path, line_number, error);
        }
        emit_result(&result);
        if result.status == ExecutionStatus::Exit {
            return None;
        }
    }
    if assembler.is_unfinished() {
        report(
            script,
            line_number,
            "the input ends inside an unfinished command",
        );
        return Some(SYNTAX_ERROR);
    }
    None
}

fn report(script: Option<&str>, line_number: usize, error: &str) {
    let _ = match script {
        Some(path) => render::stderr_line(located(path, line_number, error)),
        None => render::stderr_line(error),
    };
}

fn located(path: &str, line_number: usize, error: &str) -> String {
    format!("terminal_rust: {}:{line_number}: {error}", printable(path))
}

struct ScriptError {
    status: u8,
    reason: String,
}

fn read_script(path: &str) -> Result<String, ScriptError> {
    let cannot_run = |reason: String| ScriptError {
        status: CANNOT_RUN,
        reason,
    };
    let unreadable = |error: io::Error| ScriptError {
        status: if error.kind() == io::ErrorKind::NotFound {
            NOT_FOUND
        } else {
            CANNOT_RUN
        },
        reason: error.to_string(),
    };
    let metadata = fs::metadata(Path::new(path)).map_err(unreadable)?;
    if metadata.is_dir() {
        return Err(cannot_run("is a directory".into()));
    }
    if !metadata.is_file() {
        return Err(cannot_run("is not a regular file".into()));
    }
    if metadata.len() > MAX_SCRIPT_BYTES {
        return Err(cannot_run("is larger than 1 MiB".into()));
    }
    let bytes = fs::read(path).map_err(unreadable)?;
    String::from_utf8(bytes)
        .ok()
        .filter(|content| !content.contains('\0'))
        .ok_or_else(|| cannot_run("is not a text file".into()))
}
