use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::help;
use crate::app::parser::{Connector, Stage, parse_args, split_chain, split_stages};
use crate::commands::{
    appearance, arch, cat, cd, clear, dashboard, date, echo, editor, external, fastfetch,
    git_command, hostname, ls, man, mkdir, nproc, plugins, printenv, pwd, rm, theme, touch, uptime,
    user, whoami,
};
use crate::config::UserCommandKind;
use crate::errors::command::CommandError;
use crate::plugin::DeclarativeCommandRegistry;
use crate::render;
use crate::render::printable::printable_multiline;
use crate::utils::shell_env;

mod aliases;
mod jobs;
mod pipeline;
mod scripts;
mod substitution;

pub struct ShellState {
    previous_dir: Option<PathBuf>,
    plugin_commands: DeclarativeCommandRegistry,
    user_commands: user::UserCommandRegistry,
    startup_script: Option<String>,
    script_depth: usize,
    variables: HashMap<String, String>,
    /// The status of the last command, the value of `$?`.
    last_status: i32,
    /// The code the shell leaves with, once `exit` has run.
    exit_code: Option<u8>,
    jobs: jobs::Jobs,
}

impl ShellState {
    pub(crate) fn new(
        user_commands: user::UserCommandRegistry,
        plugin_commands: DeclarativeCommandRegistry,
        startup_script: Option<String>,
    ) -> Self {
        Self {
            previous_dir: None,
            plugin_commands,
            user_commands,
            startup_script,
            script_depth: 0,
            variables: HashMap::new(),
            last_status: 0,
            exit_code: None,
            jobs: jobs::Jobs::default(),
        }
    }

    /// Value of a variable for expansion: `?`, then shell variables, then the
    /// environment the programs would receive.
    pub(crate) fn variable(&self, name: &str) -> Option<String> {
        if name == "?" {
            return Some(self.last_status.to_string());
        }
        self.variables
            .get(name)
            .cloned()
            .or_else(|| shell_env::value(name))
    }

    /// Record the outcome of a command as `$?`: 0, or the exit code of the
    /// program that failed, or 1 for a command that failed inside the shell.
    pub(crate) fn record_status(&mut self, failed: bool) {
        let program_code = external::take_exit_code();
        self.last_status = if failed { program_code.unwrap_or(1) } else { 0 };
    }

    /// The code the shell process should exit with: the one given to `exit`, or
    /// the status of the last command.
    pub(crate) fn exit_code(&self) -> u8 {
        self.exit_code
            .unwrap_or_else(|| status_byte(i64::from(self.last_status)))
    }

    /// The notices for the background jobs that ended since the last call.
    pub(crate) fn finished_job_notices(&mut self) -> Vec<String> {
        self.jobs.finished_notices()
    }

    pub(crate) fn user_command_names(&self) -> Vec<String> {
        self.user_commands.names()
    }

    pub(crate) fn startup_script(&self) -> Option<&str> {
        self.startup_script.as_deref()
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
        name: "alias",
        usage: "alias [--save] [name command [arguments...]]",
        summary: "list, define or permanently save command aliases",
        max_args: None,
        handler: aliases::run,
    },
    CommandSpec {
        name: "unalias",
        usage: "unalias [--save] <name>",
        summary: "remove a session or permanently saved alias",
        max_args: Some(2),
        handler: aliases::remove,
    },
    CommandSpec {
        name: "script",
        usage: "script <create|run|startup|list> [arguments...]",
        summary: "create and run safe Terminal Rust command scripts",
        max_args: None,
        handler: scripts::run,
    },
    CommandSpec {
        name: "fastfetch",
        usage: "fastfetch [arguments...]",
        summary: "display system information with Fastfetch",
        max_args: None,
        handler: run_fastfetch,
    },
    CommandSpec {
        name: "ff",
        usage: "ff [arguments...]",
        summary: "alias for fastfetch",
        max_args: None,
        handler: run_fastfetch,
    },
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
        name: "appearance",
        usage: "appearance [show|theme|foreground|background|color|reset] [arguments...]",
        summary: "customize the terminal theme and ANSI colors",
        max_args: None,
        handler: run_appearance,
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
        usage: "exit [code]",
        summary: "close the terminal with an exit code",
        max_args: Some(1),
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
        name: "jobs",
        usage: "jobs",
        summary: "list the background jobs",
        max_args: Some(0),
        handler: run_jobs,
    },
    CommandSpec {
        name: "fg",
        usage: "fg [%job]",
        summary: "wait for a background job and show its result",
        max_args: Some(1),
        handler: run_fg,
    },
    CommandSpec {
        name: "wait",
        usage: "wait [%job]",
        summary: "wait for one background job, or for all of them",
        max_args: Some(1),
        handler: run_wait,
    },
    CommandSpec {
        name: "kill",
        usage: "kill %job | kill [options] pid",
        summary: "stop a background job, or send a signal to a process",
        max_args: None,
        handler: run_kill,
    },
    CommandSpec {
        name: "man",
        usage: "man <command>",
        summary: "display the manual of a command",
        max_args: None,
        handler: run_man,
    },
    CommandSpec {
        name: "export",
        usage: "export [NAME[=value]...]",
        summary: "make variables available to the programs you start",
        max_args: None,
        handler: run_export,
    },
    CommandSpec {
        name: "unset",
        usage: "unset NAME...",
        summary: "remove variables, including inherited ones",
        max_args: None,
        handler: run_unset,
    },
    CommandSpec {
        name: "hostname",
        usage: "hostname",
        summary: "display the system hostname",
        max_args: Some(0),
        handler: run_hostname,
    },
    CommandSpec {
        name: "printenv",
        usage: "printenv [variable]",
        summary: "display environment variables",
        max_args: Some(1),
        handler: run_printenv,
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

/// Run a command line: one or more commands joined by `;`, `&&` and `||`.
///
/// Each segment is expanded right before it runs, so `$?` and variables set by
/// an earlier segment are visible to the next one. The error of a segment that
/// is followed by another one is printed here; the error of the last segment
/// that ran is returned.
pub(crate) fn run_line(input: &str, state: &mut ShellState) -> CommandResult {
    let segments = match split_chain(input) {
        Ok(segments) => segments,
        Err(error) => return syntax_error(error, state),
    };
    // A syntax error anywhere rejects the whole line before anything runs.
    let mut lines = Vec::with_capacity(segments.len());
    for segment in segments {
        match split_stages(&segment.text) {
            Ok(stages) => lines.push((segment, stages)),
            Err(error) => return syntax_error(error, state),
        }
    }

    let mut last = CommandResult::Continue;
    for (segment, stages) in lines {
        let failed = matches!(last, CommandResult::Error(_));
        let run = match segment.connector {
            Connector::Then => true,
            Connector::And => !failed,
            Connector::Or => failed,
        };
        if !run {
            continue;
        }
        if let CommandResult::Error(error) = &last
            && !error.is_empty()
        {
            let _ = render::stderr_line(error);
        }

        last = run_segment(&segment.text, &stages, segment.background, state);
        state.record_status(matches!(last, CommandResult::Error(_)));
        if last == CommandResult::Exit {
            break;
        }
    }
    last
}

/// A line that cannot be parsed fails without running anything.
fn syntax_error(message: String, state: &mut ShellState) -> CommandResult {
    state.record_status(true);
    CommandResult::Error(message)
}

/// Run one segment: a single command, or a pipeline and its redirections.
fn run_segment(
    text: &str,
    stages: &[Stage],
    background: bool,
    state: &mut ShellState,
) -> CommandResult {
    // Command substitutions run now, then the rewritten text is split again.
    if text.contains("$(") {
        let expanded = match substitution::expand(text, state) {
            Ok(expanded) => expanded,
            Err(error) => return CommandResult::Error(error),
        };
        return match split_stages(&expanded) {
            Ok(stages) => run_expanded(&expanded, &stages, background, state),
            Err(error) => CommandResult::Error(error),
        };
    }
    run_expanded(text, stages, background, state)
}

fn run_expanded(
    text: &str,
    stages: &[Stage],
    background: bool,
    state: &mut ShellState,
) -> CommandResult {
    // Inside a substitution the output of programs is collected: only the
    // pipeline code does that.
    if let [stage] = stages
        && stage.redirects.is_empty()
        && !background
        && !render::programs_captured()
    {
        return match parse_args(text, &|name| state.variable(name)) {
            Ok(args) if args.is_empty() => CommandResult::Continue,
            Ok(args) => execute(&args, state),
            Err(error) => CommandResult::Error(error),
        };
    }
    pipeline::run(stages, state, background.then_some(text))
}

/// How a pipeline stage runs.
enum StageKind {
    /// Inside the shell: built-in commands, plugin commands, assignments.
    InProcess,
    /// As a program, with its streams connected to the neighbours.
    Program { program: String, args: Vec<String> },
    /// The native Git, which has its own environment (the bundled runtime).
    Git { args: Vec<String> },
}

fn classify(args: &[String], state: &ShellState) -> Result<StageKind, String> {
    let Some(name) = args.first().map(String::as_str) else {
        return Err("expected a command".to_string());
    };
    if let [single] = args
        && split_assignment(single).is_some()
    {
        return Ok(StageKind::InProcess);
    }
    // Git runs as a program, so its output can go through a pipe or a redirection.
    if name == "git" {
        return Ok(StageKind::Git {
            args: args[1..].to_vec(),
        });
    }
    if find_command(name).is_some() {
        return Ok(StageKind::InProcess);
    }
    if let Some(command) = state.user_commands.get(name) {
        return match command.kind {
            UserCommandKind::Builtin => Ok(StageKind::InProcess),
            UserCommandKind::Process => user::process_invocation(name, command, &args[1..])
                .map(|(program, args)| StageKind::Program { program, args }),
        };
    }
    if state.plugin_commands.get(name).is_some()
        || state.plugin_commands.blocked_reason(name).is_some()
    {
        return Ok(StageKind::InProcess);
    }
    Ok(StageKind::Program {
        program: name.to_string(),
        args: args[1..].to_vec(),
    })
}

/// Split the leading `NAME=value` words off a command: `FOO=1 BAR=2 cmd args`.
/// They only apply when a command follows; alone they assign shell variables.
fn split_env_prefix(args: &[String]) -> (Vec<(String, String)>, &[String]) {
    let count = args
        .iter()
        .take_while(|word| split_assignment(word).is_some())
        .count();
    if count == 0 || count == args.len() {
        return (Vec::new(), args);
    }
    let vars = args[..count]
        .iter()
        .filter_map(|word| split_assignment(word))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    (vars, &args[count..])
}

pub fn execute(args: &[String], state: &mut ShellState) -> CommandResult {
    let (env, rest) = split_env_prefix(args);
    if !env.is_empty() {
        return shell_env::with_temporary(&env, || execute(rest, state));
    }
    let Some(command_name) = args.first().map(String::as_str) else {
        return run_help(&[], state);
    };

    if let [single] = args
        && let Some((name, value)) = split_assignment(single)
    {
        return assign_variable(name, value, state);
    }

    if let Some(command) = find_command(command_name) {
        let command_args = &args[1..];
        if let Err(error) = validate_arg_count(command, command_args.len()) {
            return CommandResult::Error(error.to_string());
        }
        return (command.handler)(command_args, state);
    }
    if state.user_commands.get(command_name).is_some() {
        return execute_user_command(command_name, &args[1..], state);
    }
    execute_plugin_command(command_name, &args[1..], state)
}

fn run_fastfetch(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(fastfetch::run(args))
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
        return execute_external(name, invocation_args);
    };
    if !invocation_args.is_empty() {
        return CommandResult::Error(format!(
            "{name}: declarative command from plugin '{}' does not accept arguments",
            command.plugin_id
        ));
    }
    map_prefixed_error(name, render::stdout(printable_multiline(&command.response)))
}

fn execute_external(name: &str, args: &[String]) -> CommandResult {
    match external::run(name, args) {
        Ok(()) => CommandResult::Continue,
        Err(external::RunError::NotFound) => {
            external::note_exit_code(external::COMMAND_NOT_FOUND);
            CommandResult::Error(CommandError::UnknownCommand(name.to_string()).to_string())
        }
        Err(external::RunError::Failed(message)) => CommandResult::Error(message),
    }
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

fn run_printenv(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_prefixed_error("printenv", printenv::printenv(args))
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
            if previous_dir.is_some() {
                state.previous_dir = previous_dir;
            }
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

fn run_appearance(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(appearance::appearance(args))
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

fn run_man(args: &[String], _state: &mut ShellState) -> CommandResult {
    let Some(topic) = args.first() else {
        return CommandResult::Error(
            "man: what manual page do you want? Usage: man <command>".into(),
        );
    };
    if let (1, Some(command)) = (args.len(), find_command(topic)) {
        let entry = man::Entry {
            name: command.name,
            usage: command.usage,
            summary: command.summary,
        };
        return map_prefixed_error("man", man::show(&entry));
    }
    map_display_error(man::show_system(args))
}

fn split_assignment(argument: &str) -> Option<(&str, &str)> {
    let (name, value) = argument.split_once('=')?;
    shell_env::is_valid_name(name).then_some((name, value))
}

fn assign_variable(name: &str, value: &str, state: &mut ShellState) -> CommandResult {
    if shell_env::is_exported(name) {
        shell_env::export(name, value);
    } else {
        state.variables.insert(name.to_string(), value.to_string());
    }
    CommandResult::Continue
}

fn run_export(args: &[String], state: &mut ShellState) -> CommandResult {
    if args.is_empty() {
        return map_prefixed_error("export", printenv::printenv(&[]));
    }
    for argument in args {
        let (name, value) = match argument.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (argument.as_str(), None),
        };
        if !shell_env::is_valid_name(name) {
            return CommandResult::Error(format!("export: not a valid name: {name}"));
        }
        let value = match value {
            Some(value) => value.to_string(),
            None => state
                .variables
                .get(name)
                .cloned()
                .or_else(|| shell_env::value(name))
                .unwrap_or_default(),
        };
        state.variables.remove(name);
        shell_env::export(name, &value);
    }
    CommandResult::Continue
}

fn run_unset(args: &[String], state: &mut ShellState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("unset: expected at least one name".into());
    }
    for name in args {
        if !shell_env::is_valid_name(name) {
            return CommandResult::Error(format!("unset: not a valid name: {name}"));
        }
        state.variables.remove(name.as_str());
        shell_env::remove(name);
    }
    CommandResult::Continue
}

fn run_help(_args: &[String], _state: &mut ShellState) -> CommandResult {
    map_result(help::print_help(command_help()), |error| {
        format!("help: {error}")
    })
}

fn run_jobs(_args: &[String], state: &mut ShellState) -> CommandResult {
    for line in state.jobs.listing() {
        if let Err(error) = render::stdout_line(line) {
            return CommandResult::Error(format!("jobs: {error}"));
        }
    }
    CommandResult::Continue
}

/// The outcome of a job that was waited for: its exit code, as `$?`.
fn job_outcome(name: &str, status: Option<std::process::ExitStatus>) -> CommandResult {
    match jobs::describe(status) {
        (_, 0) => CommandResult::Continue,
        (_, code) => {
            external::note_exit_code(code);
            CommandResult::Error(format!("{name}: exited with status {code}"))
        }
    }
}

fn run_fg(args: &[String], state: &mut ShellState) -> CommandResult {
    match state.jobs.foreground(optional_arg(args)) {
        Ok((text, status)) => {
            let _ = render::stdout_line(&text);
            job_outcome("fg", status)
        }
        Err(error) => CommandResult::Error(format!("fg: {error}")),
    }
}

fn run_wait(args: &[String], state: &mut ShellState) -> CommandResult {
    match state.jobs.wait(optional_arg(args)) {
        Ok(status) => job_outcome("wait", status),
        Err(error) => CommandResult::Error(format!("wait: {error}")),
    }
}

/// `kill %n` stops a job of the shell; anything else is the system `kill`.
fn run_kill(args: &[String], state: &mut ShellState) -> CommandResult {
    match args {
        [spec] if spec.starts_with('%') => match state.jobs.kill(spec) {
            Ok(()) => CommandResult::Continue,
            Err(error) => CommandResult::Error(format!("kill: {error}")),
        },
        _ => execute_external("kill", args),
    }
}

fn run_exit(args: &[String], state: &mut ShellState) -> CommandResult {
    state.exit_code = match args.first() {
        // Without a code the shell leaves with the status of the last command.
        None => Some(status_byte(i64::from(state.last_status))),
        Some(argument) => match argument.parse::<i64>() {
            Ok(code) => Some(status_byte(code)),
            Err(_) => {
                return CommandResult::Error(format!(
                    "exit: numeric argument required: '{argument}'"
                ));
            }
        },
    };
    CommandResult::Exit
}

/// An exit code as the operating system keeps it: modulo 256.
fn status_byte(code: i64) -> u8 {
    code.rem_euclid(256) as u8
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_state() -> ShellState {
        ShellState::new(
            user::UserCommandRegistry::default(),
            DeclarativeCommandRegistry::default(),
            None,
        )
    }

    fn string_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|argument| (*argument).into()).collect()
    }

    #[test]
    fn command_metadata_is_complete_and_unique() {
        let mut names = HashSet::new();
        for command in COMMANDS {
            assert!(names.insert(command.name), "duplicate command name");
            assert!(command.usage.starts_with(command.name));
            assert!(!command.summary.trim().is_empty());
        }
        assert!(find_command("fastfetch").is_some());
        assert!(find_command("ff").is_some());
    }

    #[test]
    fn session_aliases_are_created_and_executed() {
        let mut state = test_state();
        let (result, _) = render::capture_output(|| {
            execute(&string_args(&["alias", "where", "pwd"]), &mut state)
        });
        assert!(matches!(result, CommandResult::Continue));

        let (result, output) =
            render::capture_output(|| execute(&string_args(&["where"]), &mut state));
        assert!(matches!(result, CommandResult::Continue));
        assert!(
            output.stdout.contains(
                &std::env::current_dir()
                    .expect("current directory should be available")
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn scripts_stop_on_the_first_failed_command() {
        let path = std::env::temp_dir().join(format!(
            "terminal-rust-script-{}-{}.trs",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should follow the epoch")
                .as_nanos()
        ));
        std::fs::write(
            &path,
            "echo before failure\nmissing-terminal-rust-test-command\necho after failure\n",
        )
        .expect("script fixture should be written");
        let mut state = test_state();
        let (result, output) = render::capture_output(|| {
            execute(
                &string_args(&["script", "run", &path.to_string_lossy()]),
                &mut state,
            )
        });
        assert!(
            matches!(result, CommandResult::Error(error) if error.contains(":2:") && error.contains("unknown command"))
        );
        assert!(output.stdout.contains("before failure"));
        assert!(!output.stdout.contains("after failure"));
        assert_eq!(state.script_depth, 0);
        std::fs::remove_file(path).expect("script fixture should be removed");
    }

    #[test]
    fn man_shows_the_manual_of_a_built_in_command() {
        let mut state = test_state();
        let (result, output) =
            render::capture_output(|| execute(&string_args(&["man", "printenv"]), &mut state));

        assert!(matches!(result, CommandResult::Continue));
        assert!(output.stdout.contains("NAME\n    printenv - "));
        assert!(output.stdout.contains("SYNOPSIS\n    printenv [variable]"));
    }

    #[test]
    fn man_without_a_topic_explains_how_to_use_it() {
        let mut state = test_state();
        let result = execute(&string_args(&["man"]), &mut state);

        assert!(matches!(
            result,
            CommandResult::Error(message) if message.contains("Usage: man <command>")
        ));
    }
}
