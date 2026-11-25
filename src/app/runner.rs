use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Cmd, ColorMode, CompletionType, Config, Editor, KeyEvent};

use crate::app::batch;
use crate::app::command::{command_names, input_is_incomplete};
use crate::app::completion::ShellHelper;
use crate::app::history;
use crate::app::launch::{self, Action, Invocation, Target, UsageError};
use crate::app::parser::{continuation_prefix, normalize_multiline_input};
use crate::app::prompt::{GUI_PROMPT_ENV, Prompt};
use crate::app::startup;
use crate::commands::external;
use crate::config::{self, AppConfig};
use crate::core::{ExecutionResult, ExecutionStatus, ShellEngine};
use crate::render;
use crate::utils::{job_control, stdin_line};

const HISTORY_FILE_ENV: &str = "TERMINAL_RUST_HISTORY_FILE";

pub fn run() -> ExitCode {
    render::stream_before_children(true);
    let invocation = match read_invocation() {
        Ok(invocation) => invocation,
        Err(error) => {
            let _ = render::stderr_line(format_args!("terminal_rust: {error}"));
            return ExitCode::from(2);
        }
    };

    let Invocation {
        login,
        interactive,
        action,
    } = invocation;
    if action == Action::Shell && (interactive || io::stdin().is_terminal()) {
        return interactive_shell(login);
    }
    if login {
        startup::import_login_environment();
    }
    match action {
        Action::Shell => batch::run_stdin(),
        Action::CommandString {
            command,
            name,
            args,
        } => batch::run_command_string(&command, name.as_deref(), &args, interactive),
        Action::Operands(operands) => {
            match launch::target(operands, is_command, |path| Path::new(path).is_file()) {
                Target::Script { path, args } => batch::run_script(&path, &args, interactive),
                Target::Command(args) => run_once(&args, interactive),
            }
        }
        Action::OneLine(args) => run_once(&args, interactive),
    }
}

fn read_invocation() -> Result<Invocation, UsageError> {
    let mut arguments = env::args_os();
    let argv0 = arguments
        .next()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let arguments = launch::decode(arguments)?;
    launch::parse(&argv0, &arguments)
}

/// A built-in, or a program of the search path. A match through a relative entry of the
/// path does not count: a `#!` script found that way would start itself without end.
fn is_command(name: &str) -> bool {
    command_names().any(|built_in| built_in == name)
        || external::locate(name).is_some_and(|program| program.has_root())
}

fn read_editor_command<P: rustyline::Prompt + ?Sized>(
    editor: &mut Editor<ShellHelper, DefaultHistory>,
    prompt: &P,
) -> Result<String, ReadlineError> {
    let mut input = editor.readline(prompt)?;
    input = normalize_multiline_input(&input);
    while let Some(prefix) = continuation_prefix(&input) {
        let prefix = prefix.to_string();
        input = prefix;
        let continuation = editor.readline(">> ")?;
        input.push(' ');
        input.push_str(&normalize_multiline_input(&continuation));
    }
    // A `for`, `while` or `if` goes on until its `done` or `fi`.
    while input_is_incomplete(&input) {
        let more = editor.readline(">> ")?;
        input.push('\n');
        input.push_str(&more);
    }
    Ok(input)
}

fn run_once(args: &[String], interactive: bool) -> ExitCode {
    let app_config = load_config();
    let mut engine = load_engine(&app_config);
    if interactive && let Some(status) = startup::run_rc(&mut engine) {
        return status;
    }
    let result = engine.execute_args(args);
    emit_result(&result);

    // The exact code of the program or command that ran, 1 for a failure inside
    // the shell.
    let status = engine.exit_code();
    if interactive {
        // The rc file may have set an `EXIT` trap.
        return batch::finish(&mut engine, status);
    }
    ExitCode::from(status)
}

pub fn run_interactive() -> ExitCode {
    render::stream_before_children(true);
    interactive_shell(false)
}

fn interactive_shell(login: bool) -> ExitCode {
    let on_terminal = io::stdin().is_terminal();
    // Before any thread starts, so that they all share the signal mask it sets.
    if on_terminal {
        job_control::init();
    }
    external::keep_shell_alive_on_interrupt();
    // After both, so that `Ctrl+C` ends a profile that hangs and not the shell.
    if login {
        startup::import_login_environment();
    }
    if on_terminal {
        run_interactive_editor()
    } else {
        run_interactive_plain()
    }
}

fn run_interactive_editor() -> ExitCode {
    let (mut engine, mut shell_prompt) = match load_interactive_shell() {
        Ok(shell) => shell,
        Err(status) => return status,
    };
    let mut config_builder = Config::builder().completion_type(CompletionType::Circular);
    if env::var_os(GUI_PROMPT_ENV).is_some() {
        config_builder = config_builder.color_mode(ColorMode::Forced);
    }
    let config = config_builder.build();
    let mut editor = match Editor::<ShellHelper, DefaultHistory>::with_config(config) {
        Ok(editor) => editor,
        Err(error) => {
            let _ = render::stderr_line(format_args!("terminal: {error}"));
            return ExitCode::FAILURE;
        }
    };
    // `Ctrl+Z` stops the programs that run, not the shell: at the prompt it does nothing.
    editor.bind_sequence(KeyEvent::ctrl('Z'), Cmd::Noop);
    editor.set_helper(Some(ShellHelper::with_command_names(
        engine.command_names(),
    )));

    let history_path = history_path();
    if let Some(path) = history_path.as_deref() {
        match editor.load_history(path) {
            Ok(()) => {}
            Err(ReadlineError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                let _ = render::stderr_line(format_args!(
                    "history: cannot load {}: {error}",
                    path.display()
                ));
            }
        }
    }

    engine.set_history(editor.history().iter().cloned().collect());

    let exit_code = loop {
        show_job_notices(&mut engine);
        let (prompt, styled_prompt) = match shell_prompt.render_for_editor() {
            Ok(prompt) => prompt,
            Err(error) => {
                let _ = render::stderr_line(format_args!("prompt: {error}"));
                break ExitCode::FAILURE;
            }
        };
        let editor_prompt = (prompt, styled_prompt);
        match read_editor_command(&mut editor, &editor_prompt) {
            Ok(input) => {
                // `!!`, `!$`, `!n` and `!prefix` stand for earlier lines. The
                // line that runs is shown, then recorded as it was expanded.
                let input = match history::expand(input.trim(), engine.history()) {
                    Ok(None) => input,
                    Ok(Some(expanded)) => {
                        let _ = render::stdout_line(&expanded);
                        expanded
                    }
                    Err(message) => {
                        let _ = render::stderr_line(message);
                        continue;
                    }
                };
                let trimmed = input.trim();
                if !trimmed.is_empty() && editor.add_history_entry(trimmed).unwrap_or(false) {
                    engine.record_history(trimmed);
                }
                if handle_input(&input, &mut engine, &mut shell_prompt) == ExecutionStatus::Exit {
                    break ExitCode::from(engine.exit_code());
                }
                editor.set_helper(Some(ShellHelper::with_command_names(
                    engine.command_names(),
                )));
            }
            Err(ReadlineError::Interrupted) => {
                let _ = render::stdout_line("^C");
            }
            Err(ReadlineError::Eof) => break ExitCode::from(engine.exit_code()),
            Err(error) => {
                let _ = render::stderr_line(format_args!("read: {error}"));
                break ExitCode::FAILURE;
            }
        }
    };

    finish_shell(&mut engine);

    if let Some(path) = history_path.as_deref()
        && let Err(error) = editor.save_history(path)
    {
        let _ = render::stderr_line(format_args!(
            "history: cannot save {}: {error}",
            path.display()
        ));
    }

    exit_code
}

fn history_path() -> Option<PathBuf> {
    env::var_os(HISTORY_FILE_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Say which background jobs ended, once, before the next prompt.
fn show_job_notices(engine: &mut ShellEngine) {
    for notice in engine.job_notices() {
        let _ = render::stdout_line(notice);
    }
}

/// Run the `EXIT` trap and show what it printed.
fn finish_shell(engine: &mut ShellEngine) {
    let result = engine.finish();
    emit_result(&result);
}

fn run_interactive_plain() -> ExitCode {
    let (mut engine, mut shell_prompt) = match load_interactive_shell() {
        Ok(shell) => shell,
        Err(status) => return status,
    };
    let code = plain_loop(&mut engine, &mut shell_prompt);
    finish_shell(&mut engine);
    code
}

fn plain_loop(engine: &mut ShellEngine, shell_prompt: &mut Prompt) -> ExitCode {
    let mut input = String::new();

    loop {
        show_job_notices(engine);
        if let Err(error) = shell_prompt.print() {
            let _ = render::stderr_line(format_args!("prompt: {error}"));
            return ExitCode::FAILURE;
        }

        input.clear();
        // The reader is dropped before the command runs: on a terminal it holds the
        // lock of stdin, which `read` and `rm` take again and which is not reentrant.
        let line =
            stdin_line::reader().and_then(|mut reader| read_plain_command(&mut reader, &mut input));
        match line {
            Ok(true) => {}
            Ok(false) => return ExitCode::from(engine.exit_code()),
            Err(error) => {
                let _ = render::stderr_line(format_args!("read: {error}"));
                return ExitCode::FAILURE;
            }
        }

        if handle_input(&input, engine, shell_prompt) == ExecutionStatus::Exit {
            return ExitCode::from(engine.exit_code());
        }
    }
}

/// `Err` is the status of an rc file that ended the shell before its first prompt.
fn load_interactive_shell() -> Result<(ShellEngine, Prompt), ExitCode> {
    let config = load_config();
    let mut engine = load_engine(&config);
    startup::apply_native_terminal_appearance();
    run_fastfetch_if_enabled(&mut engine);
    if let Some(status) = startup::run_rc(&mut engine) {
        return Err(status);
    }
    run_startup_script(&mut engine);
    Ok((engine, load_prompt(&config)))
}

fn read_plain_command(reader: &mut impl BufRead, input: &mut String) -> io::Result<bool> {
    if reader.read_line(input)? == 0 {
        return Ok(false);
    }
    while let Some(prefix) = continuation_prefix(input) {
        let prefix = prefix.to_string();
        *input = prefix;
        render::stdout(">> ")?;
        let mut continuation = String::new();
        if reader.read_line(&mut continuation)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "input ended during a continued command",
            ));
        }
        input.push(' ');
        input.push_str(continuation.trim_end_matches(['\r', '\n']));
    }
    // A `for`, `while` or `if` goes on until its `done` or `fi`, and a here-document
    // until its delimiter; the end of the input then leaves it unfinished, and
    // running it reports that.
    let mut first_line = true;
    while input_is_incomplete(input) {
        render::stdout(">> ")?;
        let mut more = String::new();
        if reader.read_line(&mut more)? == 0 {
            break;
        }
        // Lines are joined with exactly one line ending: the first line still has
        // its own, which would otherwise start a here-document with a blank line.
        if first_line {
            input.truncate(input.trim_end_matches(['\r', '\n']).len());
            first_line = false;
        }
        input.push('\n');
        input.push_str(more.trim_end_matches(['\r', '\n']));
    }
    Ok(true)
}

fn run_fastfetch_if_enabled(engine: &mut ShellEngine) {
    if !fastfetch_enabled(env::var("TERMINAL_RUST_FASTFETCH").ok().as_deref()) {
        return;
    }
    emit_result(&engine.execute_args(&["fastfetch".into()]));
}

fn run_startup_script(engine: &mut ShellEngine) {
    let Some(path) = engine.startup_script().map(str::to_owned) else {
        return;
    };
    emit_result(&engine.execute_args(&["script".into(), "run".into(), path]));
}

fn fastfetch_enabled(setting: Option<&str>) -> bool {
    setting == Some("1")
}

fn handle_input(input: &str, engine: &mut ShellEngine, prompt: &mut Prompt) -> ExecutionStatus {
    let started = Instant::now();
    let result = engine.execute_line(input);
    prompt.record_execution(result.status, started.elapsed());
    emit_result(&result);
    result.status
}

pub(super) fn emit_result(result: &ExecutionResult) {
    if !result.stdout.is_empty() {
        let _ = render::stdout(&result.stdout);
    }
    if !result.stderr.is_empty() {
        let _ = render::stderr(&result.stderr);
    }
    if let Some(error) = &result.error {
        let _ = render::stderr_line(error);
    }
}

pub(super) fn load_config() -> AppConfig {
    let mut app_config = match config::load_user_config() {
        Ok(config) => config,
        Err(error) => {
            let _ = render::stderr_line(format_args!("config: {error}; using defaults"));
            AppConfig::default()
        }
    };
    match config::load_persistent_shell_state() {
        Ok(state) => {
            app_config.aliases = state.aliases;
            app_config.startup_script = state.startup_script;
        }
        Err(error) => {
            let _ = render::stderr_line(format_args!(
                "shell settings: {error}; aliases and startup script disabled"
            ));
        }
    }
    app_config
}

pub(super) fn load_engine(config: &AppConfig) -> ShellEngine {
    match ShellEngine::from_config(config) {
        Ok(engine) => engine,
        Err(error) => {
            let _ = render::stderr_line(format_args!(
                "terminal: cannot initialize configured commands and plugins: {error}; using built-in commands only"
            ));
            ShellEngine::new()
        }
    }
}

fn load_prompt(config: &AppConfig) -> Prompt {
    match Prompt::from_config(config) {
        Ok(prompt) => prompt,
        Err(error) => {
            let _ = render::stderr_line(format_args!("theme: {error}; using defaults"));
            Prompt::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{fastfetch_enabled, read_plain_command};
    use std::io::Cursor;

    #[test]
    fn fastfetch_requires_explicit_opt_in() {
        assert!(fastfetch_enabled(Some("1")));
        assert!(!fastfetch_enabled(None));
        assert!(!fastfetch_enabled(Some("0")));
        assert!(!fastfetch_enabled(Some("true")));
    }

    #[test]
    fn continued_commands_join_lines_and_keep_escaped_ticks() {
        let mut reader = Cursor::new("echo hello `\nworld\n");
        let mut command = String::new();
        assert!(read_plain_command(&mut reader, &mut command).unwrap());
        assert_eq!(command, "echo hello world");

        let mut reader = Cursor::new("echo hello ``\n");
        let mut command = String::new();
        assert!(read_plain_command(&mut reader, &mut command).unwrap());
        assert_eq!(command, "echo hello ``\n");
    }

    #[test]
    fn continuation_eof_is_reported_instead_of_running_partial_input() {
        let mut reader = Cursor::new("echo unfinished `\n");
        let mut command = String::new();
        let error = read_plain_command(&mut reader, &mut command).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_here_document_is_read_with_one_line_ending_between_its_lines() {
        let mut reader = Cursor::new("cat <<EOF > out\nfirst\n\nthird\nEOF\necho next\n");
        let mut command = String::new();

        assert!(read_plain_command(&mut reader, &mut command).expect("reads"));
        assert_eq!(command, "cat <<EOF > out\nfirst\n\nthird\nEOF");

        // The line after the delimiter is the next command.
        command.clear();
        assert!(read_plain_command(&mut reader, &mut command).expect("reads"));
        assert_eq!(command.trim_end(), "echo next");
    }

    #[test]
    fn the_end_of_the_input_leaves_a_here_document_unfinished() {
        let mut reader = Cursor::new("cat <<EOF\nonly a body\n");
        let mut command = String::new();

        assert!(read_plain_command(&mut reader, &mut command).expect("reads"));
        assert_eq!(command, "cat <<EOF\nonly a body");
        assert!(crate::app::command::input_is_incomplete(&command));
    }
}
