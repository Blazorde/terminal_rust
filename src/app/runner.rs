use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::process::ExitCode;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, Config, Editor};

use crate::app::command::{self, CommandResult, ShellState};
use crate::app::completion::ShellHelper;
use crate::app::parser::parse_input;
use crate::app::prompt::{print_prompt, prompt};

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
            eprintln!("{error}");
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
            eprintln!("terminal: {error}");
            return ExitCode::FAILURE;
        }
    };
    editor.set_helper(Some(ShellHelper::new()));

    let mut state = ShellState::default();

    loop {
        let prompt = match prompt() {
            Ok(prompt) => prompt,
            Err(error) => {
                eprintln!("prompt: {error}");
                return ExitCode::FAILURE;
            }
        };

        match editor.readline(&prompt) {
            Ok(input) => {
                let trimmed = input.trim();
                if !trimmed.is_empty() {
                    let _ = editor.add_history_entry(trimmed);
                }
                if handle_input(&input, &mut state) == CommandResult::Exit {
                    return ExitCode::SUCCESS;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(ReadlineError::Eof) => return ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("read: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
}

fn run_interactive_plain() -> ExitCode {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut input = String::new();
    let mut state = ShellState::default();

    loop {
        if let Err(error) = print_prompt() {
            eprintln!("prompt: {error}");
            return ExitCode::FAILURE;
        }

        input.clear();
        match stdin.read_line(&mut input) {
            Ok(0) => return ExitCode::SUCCESS,
            Ok(_) => {}
            Err(error) => {
                eprintln!("read: {error}");
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
            eprintln!("{error}");
            return CommandResult::Continue;
        }
    };

    match command::execute(&args, state) {
        CommandResult::Error(error) => {
            eprintln!("{error}");
            CommandResult::Continue
        }
        result => result,
    }
}
