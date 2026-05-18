use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use crate::commands::{cd, ls, pwd};

pub fn run() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        run_interactive()
    } else {
        run_once(&args)
    }
}

fn run_once(args: &[String]) -> ExitCode {
    match execute_command(args) {
        CommandResult::Continue => ExitCode::SUCCESS,
        CommandResult::Exit => ExitCode::SUCCESS,
        CommandResult::Error(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_interactive() -> ExitCode {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut input = String::new();

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

        let args = match parse_input(&input) {
            Ok(args) if args.is_empty() => continue,
            Ok(args) => args,
            Err(error) => {
                eprintln!("{error}");
                continue;
            }
        };

        match execute_command(&args) {
            CommandResult::Continue => {}
            CommandResult::Exit => return ExitCode::SUCCESS,
            CommandResult::Error(error) => eprintln!("{error}"),
        }
    }
}

fn execute_command(args: &[String]) -> CommandResult {
    match Command::from_args(args) {
        Ok(Command::Pwd) => map_result(pwd::pwd(), |error| format!("pwd: {error}")),
        Ok(Command::Cd(target)) => map_result(cd::cd(target), |error| error.to_string()),
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

fn parse_input(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for character in input.chars() {
        match character {
            '"' => in_quotes = !in_quotes,
            character if character.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }

    if in_quotes {
        return Err("missing closing quote".to_string());
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

fn print_prompt() -> io::Result<()> {
    let current_dir = env::current_dir()?;
    print!("{}> ", current_dir.display());
    io::stdout().flush()
}

fn print_help() {
    println!("Available commands:");
    println!("  pwd          print current directory");
    println!("  cd [path]    change current directory");
    println!("  ls [-1aAdFhlpRrSt] [path...]");
    println!("  exit         close the terminal");
}

enum CommandResult {
    Continue,
    Exit,
    Error(String),
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
    fn parse_input_splits_words() {
        assert_eq!(
            parse_input("ls -la src"),
            Ok(vec!["ls".into(), "-la".into(), "src".into()])
        );
    }

    #[test]
    fn parse_input_ignores_extra_spaces() {
        assert_eq!(
            parse_input("  cd   target  "),
            Ok(vec!["cd".into(), "target".into()])
        );
    }

    #[test]
    fn parse_input_keeps_quoted_paths_together() {
        assert_eq!(
            parse_input(r#"cd "my folder""#),
            Ok(vec!["cd".into(), "my folder".into()])
        );
    }

    #[test]
    fn parse_input_rejects_unclosed_quotes() {
        assert_eq!(
            parse_input(r#"cd "my folder"#),
            Err("missing closing quote".to_string())
        );
    }

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
