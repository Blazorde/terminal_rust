use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Cmd, ColorMode, CompletionType, Config, Editor, KeyEvent};

use crate::app::completion::ShellHelper;
use crate::app::parser::{continuation_prefix, normalize_multiline_input};
use crate::app::prompt::{GUI_PROMPT_ENV, Prompt};
use crate::config::{self, AppConfig};
use crate::core::{ExecutionResult, ExecutionStatus, ShellEngine};
use crate::render;

const HISTORY_FILE_ENV: &str = "TERMINAL_RUST_HISTORY_FILE";

pub fn run() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        run_interactive()
    } else {
        run_once(&args)
    }
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
    Ok(input)
}

fn run_once(args: &[String]) -> ExitCode {
    let app_config = load_config();
    let mut engine = load_engine(&app_config);
    let result = engine.execute_args(args);
    emit_result(&result);

    match result.status {
        ExecutionStatus::Continue | ExecutionStatus::Exit => ExitCode::SUCCESS,
        ExecutionStatus::Failed => ExitCode::FAILURE,
    }
}

pub fn run_interactive() -> ExitCode {
    if io::stdin().is_terminal() {
        run_interactive_editor()
    } else {
        run_interactive_plain()
    }
}

fn run_interactive_editor() -> ExitCode {
    let (mut engine, mut shell_prompt) = load_interactive_shell();
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
    editor.bind_sequence(KeyEvent::ctrl('J'), Cmd::Newline);
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

    let exit_code = loop {
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
                let trimmed = input.trim();
                if !trimmed.is_empty() {
                    let _ = editor.add_history_entry(trimmed);
                }
                if handle_input(&input, &mut engine, &mut shell_prompt) == ExecutionStatus::Exit {
                    break ExitCode::SUCCESS;
                }
                editor.set_helper(Some(ShellHelper::with_command_names(
                    engine.command_names(),
                )));
            }
            Err(ReadlineError::Interrupted) => {
                let _ = render::stdout_line("^C");
            }
            Err(ReadlineError::Eof) => break ExitCode::SUCCESS,
            Err(error) => {
                let _ = render::stderr_line(format_args!("read: {error}"));
                break ExitCode::FAILURE;
            }
        }
    };

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

fn run_interactive_plain() -> ExitCode {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut input = String::new();
    let (mut engine, mut shell_prompt) = load_interactive_shell();

    loop {
        if let Err(error) = shell_prompt.print() {
            let _ = render::stderr_line(format_args!("prompt: {error}"));
            return ExitCode::FAILURE;
        }

        input.clear();
        match read_plain_command(&mut stdin, &mut input) {
            Ok(true) => {}
            Ok(false) => return ExitCode::SUCCESS,
            Err(error) => {
                let _ = render::stderr_line(format_args!("read: {error}"));
                return ExitCode::FAILURE;
            }
        }

        if handle_input(&input, &mut engine, &mut shell_prompt) == ExecutionStatus::Exit {
            return ExitCode::SUCCESS;
        }
    }
}

fn load_interactive_shell() -> (ShellEngine, Prompt) {
    let config = load_config();
    let mut engine = load_engine(&config);
    run_fastfetch_if_enabled(&mut engine);
    run_startup_script(&mut engine);
    (engine, load_prompt(&config))
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

fn emit_result(result: &ExecutionResult) {
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

fn load_config() -> AppConfig {
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

fn load_engine(config: &AppConfig) -> ShellEngine {
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
}
