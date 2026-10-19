use std::path::PathBuf;

use crate::app::help;
use crate::commands::{
    arch, cat, cd, clear, dashboard, date, echo, editor, git_command, hostname, ls, mkdir, nproc,
    plugins, pwd, rm, theme, touch, uptime, user, whoami,
};
use crate::config::UserCommandKind;
use crate::errors::command::CommandError;
use crate::plugin::DeclarativeCommandRegistry;
use crate::render;

pub struct ShellState {
    previous_dir: Option<PathBuf>,
    plugin_commands: DeclarativeCommandRegistry,
    user_commands: user::UserCommandRegistry,
}

impl ShellState {
    pub(crate) fn new(
        user_commands: user::UserCommandRegistry,
        plugin_commands: DeclarativeCommandRegistry,
    ) -> Self {
        Self {
            previous_dir: None,
            plugin_commands,
            user_commands,
        }
    }

    pub(crate) fn user_command_names(&self) -> impl Iterator<Item = &str> {
        self.user_commands.names()
    }

    pub(crate) fn plugin_command_names(&self) -> impl Iterator<Item = &str> {
        self.plugin_commands.names()
    }
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
    usage: &'static str,
    summary: &'static str,
    max_args: Option<usize>,
    handler: CommandHandler,
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "pwd",
        usage: "pwd",
        summary: "print current directory",
        max_args: Some(0),
        handler: run_pwd,
    },
    CommandSpec {
        name: "cd",
        usage: "cd [path]",
        summary: "change current directory",
        max_args: Some(1),
        handler: run_cd,
    },
    CommandSpec {
        name: "ls",
        usage: "ls [-1aAdFhlpRrSt] [path...]",
        summary: "list directory contents",
        max_args: None,
        handler: run_ls,
    },
    CommandSpec {
        name: "mkdir",
        usage: "mkdir [directory]",
        summary: "create a new directory",
        max_args: Some(1),
        handler: run_mkdir,
    },
    CommandSpec {
        name: "touch",
        usage: "touch [file]",
        summary: "create an empty file",
        max_args: Some(1),
        handler: run_touch,
    },
    CommandSpec {
        name: "cat",
        usage: "cat [file]",
        summary: "print file content",
        max_args: Some(1),
        handler: run_cat,
    },
    CommandSpec {
        name: "rm",
        usage: "rm [-dfirRv] [path...]",
        summary: "remove files or directories",
        max_args: None,
        handler: run_rm,
    },
    CommandSpec {
        name: "clear",
        usage: "clear",
        summary: "clear the terminal screen",
        max_args: Some(0),
        handler: run_clear,
    },
    CommandSpec {
        name: "dashboard",
        usage: "dashboard",
        summary: "open a live system performance dashboard",
        max_args: Some(0),
        handler: run_dashboard,
    },
    CommandSpec {
        name: "git",
        usage: "git <command> [arguments...]",
        summary: "run a native Git command",
        max_args: None,
        handler: run_git,
    },
    CommandSpec {
        name: "theme",
        usage: "theme <list|preview NAME>",
        summary: "list or preview built-in themes",
        max_args: None,
        handler: run_theme,
    },
    CommandSpec {
        name: "plugins",
        usage: "plugins <list|help>",
        summary: "list plugins or explain plugin support",
        max_args: None,
        handler: run_plugins,
    },
    CommandSpec {
        name: "plugin",
        usage: "plugin <list|help>",
        summary: "alias for plugins",
        max_args: None,
        handler: run_plugins,
    },
    CommandSpec {
        name: "commands",
        usage: "commands <list|help>",
        summary: "list custom commands or explain them",
        max_args: None,
        handler: run_commands,
    },
    CommandSpec {
        name: "help",
        usage: "help",
        summary: "show this help",
        max_args: Some(0),
        handler: run_help,
    },
    CommandSpec {
        name: "exit",
        usage: "exit",
        summary: "close the terminal",
        max_args: Some(0),
        handler: run_exit,
    },
    CommandSpec {
        name: "echo",
        usage: "echo [-n] [text...]",
        summary: "print text to the terminal",
        max_args: None,
        handler: run_echo,
    },
    CommandSpec {
        name: "whoami",
        usage: "whoami",
        summary: "display the current user",
        max_args: Some(0),
        handler: run_whoami,
    },
    CommandSpec {
        name: "arch",
        usage: "arch",
        summary: "display the system architecture",
        max_args: Some(0),
        handler: run_arch,
    },
    CommandSpec {
        name: "nproc",
        usage: "nproc",
        summary: "display the number of available processors",
        max_args: Some(0),
        handler: run_nproc,
    },
    CommandSpec {
        name: "uptime",
        usage: "uptime",
        summary: "display the system uptime",
        max_args: Some(0),
        handler: run_uptime,
    },
    CommandSpec {
        name: "date",
        usage: "date [format]",
        summary: "display the current date",
        max_args: Some(1),
        handler: run_date,
    },
    CommandSpec {
        name: "nano",
        usage: "nano [file...]",
        summary: "edit files with Nano",
        max_args: None,
        handler: run_nano,
    },
    CommandSpec {
        name: "nvim",
        usage: "nvim [file...]",
        summary: "edit files with Neovim",
        max_args: None,
        handler: run_nvim,
    },
    CommandSpec {
        name: "neovim",
        usage: "neovim [file...]",
        summary: "alias for nvim",
        max_args: None,
        handler: run_neovim,
    },
    CommandSpec {
        name: "hostname",
        usage: "hostname",
        summary: "display the system hostname",
        max_args: Some(0),
        handler: run_hostname,
    },
    CommandSpec {
        name: "quit",
        usage: "quit",
        summary: "close the terminal",
        max_args: Some(0),
        handler: run_exit,
    },
];

pub fn command_names() -> impl Iterator<Item = &'static str> {
    COMMANDS.iter().map(|command| command.name)
}

pub(crate) fn command_help() -> impl Iterator<Item = (&'static str, &'static str)> {
    COMMANDS
        .iter()
        .map(|command| (command.usage, command.summary))
}

pub fn execute(args: &[String], state: &mut ShellState) -> CommandResult {
    let Some(command_name) = args.first().map(String::as_str) else {
        return run_help(&[], state);
    };

    let Some(command) = find_command(command_name) else {
        if state.user_commands.get(command_name).is_some() {
            return execute_user_command(command_name, &args[1..], state);
        }
        return execute_plugin_command(command_name, &args[1..], state);
    };

    let command_args = &args[1..];
    if let Err(error) = validate_arg_count(command, command_args.len()) {
        return CommandResult::Error(error.to_string());
    }

    (command.handler)(command_args, state)
}

fn execute_user_command(
    name: &str,
    invocation_args: &[String],
    state: &mut ShellState,
) -> CommandResult {
    let Some(command) = state.user_commands.get(name).cloned() else {
        return CommandResult::Error(CommandError::UnknownCommand(name.to_string()).to_string());
    };
    let mut expanded_args = command.args.clone();
    if command.append_args {
        expanded_args.extend_from_slice(invocation_args);
    } else if !invocation_args.is_empty() {
        return CommandResult::Error(format!(
            "{name}: this custom command does not accept arguments"
        ));
    }

    match command.kind {
        UserCommandKind::Builtin => {
            expanded_args.insert(0, command.target);
            execute(&expanded_args, state)
        }
        UserCommandKind::Process => {
            map_display_error(user::run_process(name, &command, invocation_args))
        }
    }
}

fn execute_plugin_command(
    name: &str,
    invocation_args: &[String],
    state: &mut ShellState,
) -> CommandResult {
    let Some(command) = state.plugin_commands.get(name).cloned() else {
        if let Some(reason) = state.plugin_commands.blocked_reason(name) {
            return CommandResult::Error(reason.to_string());
        }
        return CommandResult::Error(CommandError::UnknownCommand(name.to_string()).to_string());
    };
    if !invocation_args.is_empty() {
        return CommandResult::Error(format!(
            "{name}: declarative command from plugin '{}' does not accept arguments",
            command.plugin_id
        ));
    }
    map_prefixed_error(name, render::stdout(command.response))
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

fn run_arch(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("arch", arch::arch())
}

fn run_uptime(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("uptime", uptime::uptime())
}

fn run_nproc(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("nproc", nproc::nproc())
}

fn run_date(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("date", date::date(args))
}

fn run_nano(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(editor::run("nano", "nano", args))
}

fn run_nvim(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(editor::run("nvim", "nvim", args))
}

fn run_neovim(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(editor::run("neovim", "nvim", args))
}

fn run_hostname(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("hostname", hostname::hostname())
}

fn run_whoami(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("whoami", whoami::whoami())
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

fn run_echo(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("echo", echo::echo(args))
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

fn run_commands(args: &[String], state: &mut ShellState) -> CommandResult {
    if args == ["help"] {
        return map_prefixed_error("commands", render::stdout(COMMANDS_HELP));
    }
    if args != ["list"] {
        return CommandResult::Error("commands: expected 'list' or 'help'".into());
    }
    map_prefixed_error(
        "commands",
        render::stdout(state.user_commands.render_list()),
    )
}

const COMMANDS_HELP: &str = "Custom commands are shortcuts configured in the user TOML file.\n\
Use 'commands list' to inspect configured shortcuts. Built-in shortcuts call a\n\
Terminal Rust command; process shortcuts start a configured program directly.\n\
See docs/custom-commands.md for configuration and security details.\n";

fn run_help(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_result(help::print_help(command_help()), |error| {
        format!("help: {error}")
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn command_metadata_is_complete_and_unique() {
        let mut names = HashSet::new();
        for command in COMMANDS {
            assert!(names.insert(command.name), "duplicate command name");
            assert!(command.usage.starts_with(command.name));
            assert!(!command.summary.trim().is_empty());
        }
    }
}
