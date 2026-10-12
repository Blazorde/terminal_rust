use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, Config, Editor};

use crate::app::completion::ShellHelper;
use crate::app::prompt::Prompt;
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
    let app_config = load_config();
    let mut engine = load_engine(&app_config);
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut editor = match Editor::<ShellHelper, DefaultHistory>::with_config(config) {
        Ok(editor) => editor,
        Err(error) => {
            let _ = render::stderr_line(format_args!("terminal: {error}"));
            return ExitCode::FAILURE;
        }
    };
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

    let mut shell_prompt = load_prompt(&app_config);

    let exit_code = loop {
        let prompt = match shell_prompt.render() {
            Ok(prompt) => prompt,
            Err(error) => {
                let _ = render::stderr_line(format_args!("prompt: {error}"));
                break ExitCode::FAILURE;
            }
        };

        match editor.readline(&prompt) {
            Ok(input) => {
                let trimmed = input.trim();
                if !trimmed.is_empty() {
                    let _ = editor.add_history_entry(trimmed);
                }
                if handle_input(&input, &mut engine, &mut shell_prompt) == ExecutionStatus::Exit {
                    break ExitCode::SUCCESS;
                }
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
    let app_config = load_config();
    let mut engine = load_engine(&app_config);
    let mut shell_prompt = load_prompt(&app_config);

    loop {
        if let Err(error) = shell_prompt.print() {
            let _ = render::stderr_line(format_args!("prompt: {error}"));
            return ExitCode::FAILURE;
        }

        input.clear();
        match stdin.read_line(&mut input) {
            Ok(0) => return ExitCode::SUCCESS,
            Ok(_) => {}
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
    match config::load_user_config() {
        Ok(config) => config,
        Err(error) => {
            let _ = render::stderr_line(format_args!("config: {error}; using defaults"));
            AppConfig::default()
        }
    }
}

fn load_engine(config: &AppConfig) -> ShellEngine {
    match ShellEngine::from_config(config) {
        Ok(engine) => engine,
        Err(error) => {
            let _ = render::stderr_line(format_args!(
                "config: invalid custom commands: {error}; custom commands disabled"
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
