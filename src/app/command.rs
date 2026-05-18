use std::path::PathBuf;

use crate::commands::{cd, ls, pwd};

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
        Ok(Command::Help) => {
            print_help();
            CommandResult::Continue
        }
        Ok(Command::Exit) => CommandResult::Exit,
        Err(error) => CommandResult::Error(error),
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

fn print_help() {
    println!("Available commands:");
    println!("  pwd          print current directory");
    println!("  cd [path]    change current directory");
    println!("  ls [-1aAdFhlpRrSt] [path...]");
    println!("  exit         close the terminal");
}

enum Command<'a> {
    Pwd,
    Cd(Option<&'a str>),
    Ls(&'a [String]),
    Help,
    Exit,
}

impl<'a> Command<'a> {
    fn from_args(args: &'a [String]) -> Result<Self, String> {
        match args.first().map(String::as_str) {
            Some("pwd") if args.len() == 1 => Ok(Self::Pwd),
            Some("pwd") => Err("pwd: too many arguments".to_string()),
            Some("cd") if args.len() <= 2 => Ok(Self::Cd(args.get(1).map(String::as_str))),
            Some("cd") => Err("cd: too many arguments".to_string()),
            Some("ls") => Ok(Self::Ls(&args[1..])),
            Some("help") => Ok(Self::Help),
            Some("exit") | Some("quit") => Ok(Self::Exit),
            Some(command) => Err(format!("unknown command: {command}")),
            None => Ok(Self::Help),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_rejects_extra_pwd_args() {
        let args = vec!["pwd".to_string(), "extra".to_string()];

        assert_eq!(
            Command::from_args(&args).err(),
            Some("pwd: too many arguments".to_string())
        );
    }

    #[test]
    fn command_rejects_extra_cd_args() {
        let args = vec!["cd".to_string(), "a".to_string(), "b".to_string()];

        assert_eq!(
            Command::from_args(&args).err(),
            Some("cd: too many arguments".to_string())
        );
    }
}
