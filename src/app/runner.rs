use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, Config, Editor};

use crate::app::command::{self, CommandResult, ShellState};
use crate::app::completion::ShellHelper;
use crate::app::parser::parse_input;
use crate::app::prompt::Prompt;
use crate::config;
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
    let mut state = ShellState::default();

    match command::execute(args, &mut state) {
        CommandResult::Continue | CommandResult::Exit => ExitCode::SUCCESS,
        CommandResult::Error(error) => {
            let _ = render::stderr_line(error);
            ExitCode::FAILURE
        }
    }
}

fn run_interactive() -> ExitCode {
    if io::stdin().is_terminal() {
        run_interactive_editor()
    } else {
        run_interactive_plain()
    }
}

fn run_interactive_editor() -> ExitCode {
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
    editor.set_helper(Some(ShellHelper::new()));

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

    let mut state = ShellState::default();
    let mut shell_prompt = load_prompt();

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
                if handle_input(&input, &mut state) == CommandResult::Exit {
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
    let mut state = ShellState::default();
    let mut shell_prompt = load_prompt();

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

        if handle_input(&input, &mut state) == CommandResult::Exit {
            return ExitCode::SUCCESS;
        }
    }
}

fn handle_input(input: &str, state: &mut ShellState) -> CommandResult {
    let args = match parse_input(input) {
        Ok(args) if args.is_empty() => return CommandResult::Continue,
        Ok(args) => args,
        Err(error) => {
            let _ = render::stderr_line(error);
            return CommandResult::Continue;
        }
    };

    match command::execute(&args, state) {
        CommandResult::Error(error) => {
            let _ = render::stderr_line(error);
            CommandResult::Continue
        }
        result => result,
    }
}

fn load_prompt() -> Prompt {
    match config::load_user_config() {
        Ok(config) => Prompt::from_config(&config),
        Err(error) => {
            let _ = render::stderr_line(format_args!("config: {error}; using defaults"));
            Prompt::default()
        }
    }
}
