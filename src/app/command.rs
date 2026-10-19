use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::app::help;
use crate::app::parser::{continuation_prefix, parse_input};
use crate::commands::{
    arch, cat, cd, clear, dashboard, date, echo, editor, fastfetch, git_command, hostname, ls,
    mkdir, nproc, plugins, pwd, rm, theme, touch, uptime, user, whoami,
};
use crate::config::{self, UserCommandConfig, UserCommandKind};
use crate::errors::command::CommandError;
use crate::plugin::DeclarativeCommandRegistry;
use crate::render;

pub struct ShellState {
    previous_dir: Option<PathBuf>,
    plugin_commands: DeclarativeCommandRegistry,
    user_commands: user::UserCommandRegistry,
    startup_script: Option<String>,
    script_depth: usize,
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
        }
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
        handler: run_alias,
    },
    CommandSpec {
        name: "unalias",
        usage: "unalias [--save] <name>",
        summary: "remove a session or permanently saved alias",
        max_args: Some(2),
        handler: run_unalias,
    },
    CommandSpec {
        name: "script",
        usage: "script <create|run|startup|list> [arguments...]",
        summary: "create and run safe Terminal Rust command scripts",
        max_args: None,
        handler: run_script,
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

fn run_alias(args: &[String], state: &mut ShellState) -> CommandResult {
    let persistent = args.first().is_some_and(|arg| arg == "--save");
    let args = if persistent { &args[1..] } else { args };
    if args.is_empty() || args == ["--list"] {
        return map_prefixed_error(
            "alias",
            render::stdout(state.user_commands.render_aliases()),
        );
    }
    if args.len() < 2 {
        return CommandResult::Error(
            "alias: expected [--save] <name> <command> [arguments...]".into(),
        );
    }

    let name = &args[0];
    let target = &args[1];
    let command = UserCommandConfig {
        kind: if find_command(target).is_some() {
            UserCommandKind::Builtin
        } else {
            UserCommandKind::Process
        },
        target: target.clone(),
        args: args[2..].to_vec(),
        append_args: true,
        description: String::new(),
    };
    let native_names = alias_reserved_names(state);
    if persistent {
        let mut aliases = state.user_commands.persistent_aliases().clone();
        aliases.insert(name.clone(), command);
        if let Err(error) = validate_aliases(&aliases, &native_names, &state.user_commands) {
            return CommandResult::Error(format!("alias: {error}"));
        }
        if let Err(error) = config::save_aliases(aliases.clone()) {
            return CommandResult::Error(format!("alias: {error}"));
        }
        if let Err(error) = state
            .user_commands
            .set_persistent_aliases(aliases, &native_names)
        {
            return CommandResult::Error(format!("alias: {error}"));
        }
    } else if let Err(error) =
        state
            .user_commands
            .set_session_alias(name.clone(), command, &native_names)
    {
        return CommandResult::Error(format!("alias: {error}"));
    }
    map_prefixed_error(
        "alias",
        render::stdout(format!(
            "Alias '{name}' saved{}.\n",
            if persistent {
                " permanently"
            } else {
                " for this session"
            }
        )),
    )
}

fn run_unalias(args: &[String], state: &mut ShellState) -> CommandResult {
    let persistent = args.first().is_some_and(|arg| arg == "--save");
    let name = if persistent {
        args.get(1)
    } else {
        args.first()
    };
    let Some(name) = name else {
        return CommandResult::Error("unalias: expected [--save] <name>".into());
    };
    if args.len() != if persistent { 2 } else { 1 } {
        return CommandResult::Error("unalias: expected [--save] <name>".into());
    }

    if persistent {
        let mut aliases = state.user_commands.persistent_aliases().clone();
        if !aliases.contains_key(name) {
            return CommandResult::Error(format!("unalias: saved alias '{name}' does not exist"));
        }
        aliases.remove(name);
        if let Err(error) = config::save_aliases(aliases.clone()) {
            return CommandResult::Error(format!("unalias: {error}"));
        }
        let native_names = alias_reserved_names(state);
        if let Err(error) = state
            .user_commands
            .set_persistent_aliases(aliases, &native_names)
        {
            return CommandResult::Error(format!("unalias: {error}"));
        }
        state.user_commands.remove_session_alias(name);
    } else if let Err(error) = state.user_commands.remove_alias(name, false) {
        return CommandResult::Error(format!("unalias: {error}"));
    }
    map_prefixed_error(
        "unalias",
        render::stdout(format!("Alias '{name}' removed.\n")),
    )
}

fn validate_aliases(
    aliases: &std::collections::BTreeMap<String, UserCommandConfig>,
    native_names: &HashSet<String>,
    commands: &user::UserCommandRegistry,
) -> Result<(), String> {
    for (name, command) in aliases {
        if commands.configured_command_exists(name) {
            return Err(format!(
                "alias '{name}' conflicts with a configured command"
            ));
        }
        if native_names.contains(name) {
            return Err(format!("alias '{name}' conflicts with an existing command"));
        }
        if !crate::config::is_valid_user_command_name(name) {
            return Err(format!("invalid alias name '{name}'"));
        }
        if command.target.trim().is_empty() || command.target.contains('\0') {
            return Err(format!("alias '{name}' has an invalid target"));
        }
        if command.kind == UserCommandKind::Builtin && !native_names.contains(&command.target) {
            return Err(format!(
                "alias '{name}' targets unknown built-in command '{}'",
                command.target
            ));
        }
        if command.args.iter().any(|argument| argument.contains('\0')) {
            return Err(format!("alias '{name}' contains a NUL argument"));
        }
    }
    Ok(())
}

fn alias_reserved_names(state: &ShellState) -> HashSet<String> {
    command_names()
        .map(str::to_string)
        .chain(state.plugin_command_names().map(str::to_string))
        .collect()
}

fn run_script(args: &[String], state: &mut ShellState) -> CommandResult {
    let Some(action) = args.first().map(String::as_str) else {
        return CommandResult::Error("script: expected create, run, startup or list".into());
    };
    match action {
        "create" if args.len() == 2 => create_script(&args[1]),
        "run" if args.len() == 2 => execute_script(&args[1], state),
        "startup" if args.len() == 2 => set_startup_script(&args[1], state),
        "list" if args.len() == 1 => list_scripts(),
        "create" | "run" | "startup" | "list" => {
            CommandResult::Error("script: invalid arguments; run 'help' for usage".into())
        }
        _ => CommandResult::Error(format!("script: unknown action '{action}'")),
    }
}

fn create_script(name: &str) -> CommandResult {
    if !crate::config::is_valid_user_command_name(name) {
        return CommandResult::Error(
            "script: name must contain lowercase letters, digits, '-' or '_'".into(),
        );
    }
    let path = PathBuf::from("scripts").join(format!("{name}.trs"));
    if let Err(error) = std::fs::create_dir_all("scripts") {
        return CommandResult::Error(format!("script: cannot create scripts directory: {error}"));
    }
    let content = format!(
        "# Terminal Rust script: {name}\n# One command per line. Lines ending in ` continue on the next line.\n\n"
    );
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => match std::io::Write::write_all(&mut file, content.as_bytes()) {
            Ok(()) => map_prefixed_error(
                "script",
                render::stdout(format!("Created {}.\n", path.display())),
            ),
            Err(error) => {
                CommandResult::Error(format!("script: cannot write {}: {error}", path.display()))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            CommandResult::Error(format!("script: {} already exists", path.display()))
        }
        Err(error) => {
            CommandResult::Error(format!("script: cannot create {}: {error}", path.display()))
        }
    }
}

fn list_scripts() -> CommandResult {
    let entries = match std::fs::read_dir("scripts") {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return map_prefixed_error("script", render::stdout("No scripts found.\n"));
        }
        Err(error) => return CommandResult::Error(format!("script: cannot list scripts: {error}")),
    };
    let mut scripts = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                return CommandResult::Error(format!(
                    "script: cannot read scripts directory entry: {error}"
                ));
            }
        };
        let path = entry.path();
        if is_script_file(&path) {
            scripts.push(path);
        }
    }
    scripts.sort();
    let output = if scripts.is_empty() {
        "No scripts found.\n".to_string()
    } else {
        scripts
            .iter()
            .map(|path| format!("{}\n", path.display()))
            .collect()
    };
    map_prefixed_error("script", render::stdout(output))
}

fn execute_script(path: &str, state: &mut ShellState) -> CommandResult {
    const MAX_SCRIPT_DEPTH: usize = 16;
    const MAX_SCRIPT_BYTES: u64 = 1_048_576;
    if state.script_depth >= MAX_SCRIPT_DEPTH {
        return CommandResult::Error("script: maximum nesting depth exceeded".into());
    }
    let script_path = PathBuf::from(path);
    let metadata = match std::fs::metadata(&script_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return CommandResult::Error(format!("script: {}: {error}", script_path.display()));
        }
    };
    if !metadata.is_file() || metadata.len() > MAX_SCRIPT_BYTES {
        return CommandResult::Error(format!(
            "script: {} must be a file no larger than 1 MiB",
            script_path.display()
        ));
    }
    if !is_script_file(&script_path) {
        return CommandResult::Error("script: expected a .trs file".into());
    }
    let content = match std::fs::read_to_string(&script_path) {
        Ok(content) => content,
        Err(error) => {
            return CommandResult::Error(format!(
                "script: cannot read {}: {error}",
                script_path.display()
            ));
        }
    };

    state.script_depth += 1;
    let mut pending = String::new();
    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(prefix) = continuation_prefix(line) {
            pending.push_str(prefix);
            pending.push(' ');
            continue;
        }
        pending.push_str(line);
        let result = match parse_input(&pending) {
            Ok(args) if args.is_empty() => CommandResult::Continue,
            Ok(args) => execute(&args, state),
            Err(error) => CommandResult::Error(error),
        };
        pending.clear();
        match result {
            CommandResult::Continue => {}
            CommandResult::Exit => {
                state.script_depth -= 1;
                return CommandResult::Exit;
            }
            CommandResult::Error(error) => {
                state.script_depth -= 1;
                return CommandResult::Error(format!(
                    "script: {}:{line_number}: {error}",
                    script_path.display()
                ));
            }
        }
    }
    state.script_depth -= 1;
    if !pending.is_empty() {
        return CommandResult::Error(format!(
            "script: {} ends after a continuation marker",
            script_path.display()
        ));
    }
    CommandResult::Continue
}

fn set_startup_script(path: &str, state: &mut ShellState) -> CommandResult {
    let startup_script = if path == "off" {
        None
    } else {
        let script_path = match std::fs::canonicalize(path) {
            Ok(path) => path,
            Err(error) => {
                return CommandResult::Error(format!("script: cannot use {path}: {error}"));
            }
        };
        if !script_path.is_file() || !is_script_file(&script_path) {
            return CommandResult::Error(
                "script: startup target must be an existing .trs file".into(),
            );
        }
        Some(script_path.to_string_lossy().into_owned())
    };
    if let Err(error) = config::save_startup_script(startup_script.clone()) {
        return CommandResult::Error(format!("script: {error}"));
    }
    state.startup_script = startup_script;
    let message = match state.startup_script.as_deref() {
        Some(path) => format!("Startup script set to {path}.\n"),
        None => "Startup script disabled.\n".into(),
    };
    map_prefixed_error("script", render::stdout(message))
}

fn run_fastfetch(args: &[String], _state: &mut ShellState) -> CommandResult {
    map_display_error(fastfetch::run(args))
}

fn is_script_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("trs"))
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
}
