use std::path::PathBuf;

use crate::app::help;
use crate::commands::{
    cat, cd, clear, dashboard, git_command, ls, mkdir, plugins, pwd, rm, theme, touch,
};
use crate::errors::command::CommandError;
use crate::render;

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

type CommandHandler = fn(&[String], &mut ShellState) -> CommandResult;

struct CommandSpec {
    name: &'static str,
    max_args: Option<usize>,
    handler: CommandHandler,
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "pwd",
        max_args: Some(0),
        handler: run_pwd,
    },
    CommandSpec {
        name: "cd",
        max_args: Some(1),
        handler: run_cd,
    },
    CommandSpec {
        name: "ls",
        max_args: None,
        handler: run_ls,
    },
    CommandSpec {
        name: "mkdir",
        max_args: Some(1),
        handler: run_mkdir,
    },
    CommandSpec {
        name: "touch",
        max_args: Some(1),
        handler: run_touch,
    },
    CommandSpec {
        name: "cat",
        max_args: Some(1),
        handler: run_cat,
    },
    CommandSpec {
        name: "rm",
        max_args: None,
        handler: run_rm,
    },
    CommandSpec {
        name: "clear",
        max_args: Some(0),
        handler: run_clear,
    },
    CommandSpec {
        name: "dashboard",
        max_args: Some(0),
        handler: run_dashboard,
    },
    CommandSpec {
        name: "git",
        max_args: None,
        handler: run_git,
    },
    CommandSpec {
        name: "theme",
        max_args: None,
        handler: run_theme,
    },
    CommandSpec {
        name: "plugins",
        max_args: None,
        handler: run_plugins,
    },
    CommandSpec {
        name: "help",
        max_args: Some(0),
        handler: run_help,
    },
    CommandSpec {
        name: "exit",
        max_args: Some(0),
        handler: run_exit,
    },
    CommandSpec {
        name: "quit",
        max_args: Some(0),
        handler: run_exit,
    },
];

pub fn command_names() -> impl Iterator<Item = &'static str> {
    COMMANDS.iter().map(|command| command.name)
}

pub fn execute(args: &[String], state: &mut ShellState) -> CommandResult {
    let Some(command_name) = args.first().map(String::as_str) else {
        return run_help(&[], state);
    };

    let Some(command) = find_command(command_name) else {
        return CommandResult::Error(
            CommandError::UnknownCommand(command_name.to_string()).to_string(),
        );
    };

    let command_args = &args[1..];
    if let Err(error) = validate_arg_count(command, command_args.len()) {
        return CommandResult::Error(error.to_string());
    }

    (command.handler)(command_args, state)
}

fn find_command(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|command| command.name == name)
}

fn validate_arg_count(command: &CommandSpec, arg_count: usize) -> Result<(), CommandError> {
    if command
        .max_args
        .is_some_and(|max_args| arg_count > max_args)
    {
        return Err(CommandError::TooManyArguments {
            command: command.name,
        });
    }

    Ok(())
}

fn optional_arg(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

fn run_pwd(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("pwd", pwd::pwd())
}

fn run_cd(args: &[String], state: &mut ShellState) -> CommandResult {
    match cd::cd(optional_arg(args), state.previous_dir.as_deref()) {
        Ok(previous_dir) => {
            state.previous_dir = Some(previous_dir);
            CommandResult::Continue
        }
        Err(error) => CommandResult::Error(error.to_string()),
    }
}

fn run_ls(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("ls", ls::ls(args))
}

fn run_mkdir(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(mkdir::mkdir(optional_arg(args)))
}

fn run_touch(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(touch::touch(optional_arg(args)))
}

fn run_cat(args: &[String], _state: &mut ShellState) -> CommandResult {
    match cat::cat(optional_arg(args)) {
        Ok(content) => map_result(render::stdout(content), |error| format!("cat: {error}")),
        Err(error) => CommandResult::Error(error.to_string()),
    }
}

fn run_rm(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(rm::rm(args))
}

fn run_clear(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("clear", clear::clear())
}

fn run_dashboard(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("dashboard", dashboard::dashboard())
}

fn run_git(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(git_command::git(args))
}

fn run_theme(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(theme::theme(args))
}

fn run_plugins(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(plugins::plugins(args))
}

fn run_help(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_result(help::print_help(), |error| format!("help: {error}"))
}

fn run_exit(_args: &[String], _state: &mut ShellState) -> CommandResult {
    CommandResult::Exit
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

fn map_display_error<T, E>(result: Result<T, E>) -> CommandResult
where
    E: std::fmt::Display,
{
    map_result(result, |error| error.to_string())
}

fn map_prefixed_error<T, E>(prefix: &str, result: Result<T, E>) -> CommandResult
where
    E: std::fmt::Display,
{
    map_result(result, |error| format!("{prefix}: {error}"))
}
