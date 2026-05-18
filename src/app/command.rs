use std::path::PathBuf;

use crate::app::help;
use crate::commands::{cd, ls, mkdir, pwd, touch};
use crate::errors::command::CommandError;

#[derive(Default)]
pub struct ShellState {
    previous_dir: Option<PathBuf>,
}

#[derive(Eq, PartialEq)]
pub enum CommandResult {
    Continue,
    Exit,
    Error(String),
}

pub fn execute(args: &[String], state: &mut ShellState) -> CommandResult {
    match Command::from_args(args) {
        Ok(Command::Pwd) => map_result(pwd::pwd(), |error| format!("pwd: {error}")),
        Ok(Command::Cd(target)) => match cd::cd(target, state.previous_dir.as_deref()) {
            Ok(previous_dir) => {
                state.previous_dir = Some(previous_dir);
                CommandResult::Continue
            }
            Err(error) => CommandResult::Error(error.to_string()),
        },
        Ok(Command::Ls(args)) => map_result(ls::ls(args), |error| format!("ls: {error}")),
        Ok(Command::Mkdir(target)) => map_result(mkdir::mkdir(target), |error| error.to_string()),
        Ok(Command::Touch(target)) => map_result(touch::touch(target), |error| error.to_string()),
        Ok(Command::Help) => {
            help::print_help();
            CommandResult::Continue
        }
        Ok(Command::Exit) => CommandResult::Exit,
        Err(error) => CommandResult::Error(error.to_string()),
    }
}

fn map_result<T, E, F>(result: Result<T, E>, format_error: F) -> CommandResult
where
    F: FnOnce(E) -> String,
{
    match result {
        Ok(_) => CommandResult::Continue,
        Err(error) => CommandResult::Error(format_error(error)),
    }
}

enum Command<'a> {
    Pwd,
    Cd(Option<&'a str>),
    Ls(&'a [String]),
    Mkdir(Option<&'a str>),
    Touch(Option<&'a str>),
    Help,
    Exit,
}

impl<'a> Command<'a> {
    fn from_args(args: &'a [String]) -> Result<Self, CommandError> {
        match args.first().map(String::as_str) {
            Some("pwd") if args.len() == 1 => Ok(Self::Pwd),
            Some("pwd") => Err(CommandError::TooManyArguments { command: "pwd" }),
            Some("cd") if args.len() <= 2 => Ok(Self::Cd(args.get(1).map(String::as_str))),
            Some("cd") => Err(CommandError::TooManyArguments { command: "cd" }),
            Some("ls") => Ok(Self::Ls(&args[1..])),
            Some("mkdir") if args.len() <= 2 => Ok(Self::Mkdir(args.get(1).map(String::as_str))),
            Some("mkdir") => Err(CommandError::TooManyArguments { command: "mkdir" }),
            Some("touch") if args.len() <= 2 => Ok(Self::Touch(args.get(1).map(String::as_str))),
            Some("touch") => Err(CommandError::TooManyArguments { command: "touch" }),
            Some("help") => Ok(Self::Help),
            Some("exit") | Some("quit") => Ok(Self::Exit),
            Some(command) => Err(CommandError::UnknownCommand(command.to_string())),
            None => Ok(Self::Help),
        }
    }
}
