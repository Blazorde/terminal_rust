use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::help;
use crate::app::parser::{Stage, parse_args, split_chain, split_stages};
use crate::commands::{
    appearance, arch, cat, cd, clear, dashboard, date, echo, editor, external, fastfetch,
    git_command, hostname, ls, man, mkdir, nproc, plugins, printenv, pwd, read_command, rm,
    test_command, theme, touch, uptime, user, whoami,
};
use crate::config::UserCommandKind;
use crate::errors::command::CommandError;
use crate::plugin::DeclarativeCommandRegistry;
use crate::render;
use crate::render::printable::printable_multiline;
use crate::utils::shell_env;

mod aliases;
mod arithmetic;
mod braces;
mod double_bracket;
mod flow;
mod functions;
mod heredoc;
mod jobs;
mod pipeline;
mod scripts;
mod substitution;

/// The state a subshell restores when it ends.
struct SubshellSnapshot {
    variables: HashMap<String, String>,
    functions: HashMap<String, functions::Body>,
    frames: Vec<functions::Frame>,
    previous_dir: Option<PathBuf>,
    directory: Option<PathBuf>,
    exports: shell_env::Exports,
}

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
    /// How many loops are running, and what `break` or `continue` asked for.
    loop_depth: u32,
    loop_signal: Option<flow::LoopSignal>,
    /// The lines typed at the prompt, oldest first (see `history`).
    history: Vec<String>,
    /// The functions defined so far (see ADR-0021).
    functions: HashMap<String, functions::Body>,
    /// The calls that are open, the innermost last: they hold `$1`, `$@` and the
    /// variables that are `local`.
    frames: Vec<functions::Frame>,
    /// Set by `return`: the status to leave the running function with.
    returning: Option<i32>,
    /// `exit` already warned about stopped jobs.
    exit_warned: bool,
    /// The process id of the last job started with `&`, for `$!`.
    pub(crate) last_background: Option<u32>,
    /// The command that `trap 'command' EXIT` runs when the shell ends.
    exit_trap: Option<String>,
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
            loop_depth: 0,
            loop_signal: None,
            history: Vec::new(),
            functions: HashMap::new(),
            frames: Vec::new(),
            returning: None,
            exit_warned: false,
            last_background: None,
            exit_trap: None,
        }
    }

    /// The names of the functions, for completion.
    pub(crate) fn function_names(&self) -> impl Iterator<Item = &str> {
        self.functions.keys().map(String::as_str)
    }

    /// The lines typed at the prompt so far, oldest first.
    pub(crate) fn history(&self) -> &[String] {
        &self.history
    }

    /// Replace the history, for example with the lines of the history file.
    pub(crate) fn set_history(&mut self, entries: Vec<String>) {
        self.history = entries;
    }

    /// Remember a line that was typed at the prompt.
    pub(crate) fn record_history(&mut self, line: &str) {
        self.history.push(line.to_string());
    }

    /// Value of a variable for expansion: `?`, then shell variables, then the
    /// environment the programs would receive.
    pub(crate) fn variable(&self, name: &str) -> Option<String> {
        if name == "?" {
            return Some(self.last_status.to_string());
        }
        // The process id of the shell, and of the last job started with `&`.
        if name == "$" {
            return Some(std::process::id().to_string());
        }
        if name == "!" {
            return Some(
                self.last_background
                    .map(|pid| pid.to_string())
                    .unwrap_or_default(),
            );
        }
        // The parent of the shell, and a number from 0 to 32767 that changes at each use.
        // A variable of the same name that the user set wins.
        if name == "PPID" && !self.variables.contains_key("PPID") {
            return Some(parent_process_id());
        }
        if name == "RANDOM" && !self.variables.contains_key("RANDOM") {
            return Some(next_random().to_string());
        }
        // The positional parameters of the running function, or of the file that
        // `source` is reading. Outside any: none, and `$#` is 0.
        let positional = self
            .frames
            .last()
            .map(|frame| frame.positional.as_slice())
            .unwrap_or_default();
        match name {
            "#" => return Some(positional.len().to_string()),
            "*" => return Some(positional.join(" ")),
            "@" => {
                return Some(positional.join(&crate::app::parser::PARAMETER_SEPARATOR.to_string()));
            }
            "0" => return Some("terminal_rust".to_string()),
            digits if digits.bytes().all(|byte| byte.is_ascii_digit()) && !digits.is_empty() => {
                return digits
                    .parse::<usize>()
                    .ok()
                    .and_then(|number| positional.get(number.checked_sub(1)?).cloned());
            }
            _ => {}
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
    /// What a subshell must give back: see [`flow::run_subshell`].
    fn subshell_snapshot(&self) -> SubshellSnapshot {
        SubshellSnapshot {
            variables: self.variables.clone(),
            functions: self.functions.clone(),
            frames: self.frames.clone(),
            previous_dir: self.previous_dir.clone(),
            directory: std::env::current_dir().ok(),
            exports: shell_env::snapshot(),
        }
    }

    fn subshell_restore(&mut self, saved: SubshellSnapshot) {
        self.variables = saved.variables;
        self.functions = saved.functions;
        self.frames = saved.frames;
        self.previous_dir = saved.previous_dir;
        if let Some(directory) = saved.directory {
            let _ = std::env::set_current_dir(directory);
        }
        shell_env::restore(saved.exports);
        // A `break`, `continue` or `return` inside the subshell ends there.
        self.loop_signal = None;
        self.returning = None;
    }

    /// The `EXIT` trap, taken so that it runs once.
    pub(crate) fn take_exit_trap(&mut self) -> Option<String> {
        self.exit_trap.take()
    }

    pub(crate) fn adopt_stopped_jobs(&mut self) {
        for notice in self.jobs.adopt_stopped() {
            let _ = render::stderr_line(notice);
        }
    }

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
        usage: "pwd [-L|-P]",
        summary: "print current directory",
        max_args: Some(1),
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
        usage: "mkdir [-p] <directory>...",
        summary: "create new directories (-p: also the missing parents)",
        max_args: None,
        handler: run_mkdir,
    },
    CommandSpec {
        name: "touch",
        usage: "touch <file>...",
        summary: "create empty files, or update their modification time",
        max_args: None,
        handler: run_touch,
    },
    CommandSpec {
        name: "cat",
        usage: "cat [file]...",
        summary: "print file content",
        max_args: None,
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
        name: "return",
        usage: "return [status]",
        summary: "leave the running function with a status",
        max_args: None,
        handler: functions::run_return,
    },
    CommandSpec {
        name: "shift",
        usage: "shift [count]",
        summary: "drop the first positional parameters ($1, $2, ...)",
        max_args: None,
        handler: functions::run_shift,
    },
    CommandSpec {
        name: "local",
        usage: "local NAME[=value]...",
        summary: "make variables local to the running function",
        max_args: None,
        handler: functions::run_local,
    },
    CommandSpec {
        name: "history",
        usage: "history [count]",
        summary: "list the commands typed at the prompt (see !!, !$, !n, !prefix)",
        max_args: Some(1),
        handler: run_history,
    },
    CommandSpec {
        name: "source",
        usage: "source <file>",
        summary: "run the commands of a file in this shell",
        max_args: None,
        handler: run_source,
    },
    CommandSpec {
        name: ".",
        usage: ". <file>",
        summary: "same as source",
        max_args: None,
        handler: run_source,
    },
    CommandSpec {
        name: "read",
        usage: "read [-r] [-p prompt] [name...]",
        summary: "read a line into variables",
        max_args: None,
        handler: run_read,
    },
    CommandSpec {
        name: "type",
        usage: "type <name>...",
        summary: "say what a name runs: keyword, built-in, alias, plugin or program",
        max_args: None,
        handler: run_type,
    },
    CommandSpec {
        name: "trap",
        usage: "trap [command] EXIT | trap - EXIT | trap",
        summary: "run a command when the shell ends",
        max_args: None,
        handler: run_trap,
    },
    CommandSpec {
        name: "command",
        usage: "command [-v|-V] name [args...]",
        summary: "run a command without its function, or say what a name is",
        max_args: None,
        handler: run_command,
    },
    CommandSpec {
        name: "exec",
        usage: "exec [command [args...]]",
        summary: "run a command and leave the shell with its status",
        max_args: None,
        handler: run_exec,
    },
    CommandSpec {
        name: "which",
        usage: "which <name>...",
        summary: "same as type",
        max_args: None,
        handler: run_type,
    },
    CommandSpec {
        name: "test",
        usage: "test <expression>",
        summary: "check a condition on files, strings or integers (status 0 if true)",
        max_args: None,
        handler: run_test,
    },
    CommandSpec {
        name: "[",
        usage: "[ <expression> ]",
        summary: "same as test, with a closing ]",
        max_args: None,
        handler: run_bracket,
    },
    CommandSpec {
        name: "true",
        usage: "true",
        summary: "do nothing, successfully",
        max_args: None,
        handler: run_true,
    },
    CommandSpec {
        name: "false",
        usage: "false",
        summary: "do nothing, unsuccessfully",
        max_args: None,
        handler: run_false,
    },
    CommandSpec {
        name: "break",
        usage: "break",
        summary: "leave the innermost for, while or until loop",
        max_args: Some(0),
        handler: run_break,
    },
    CommandSpec {
        name: "continue",
        usage: "continue",
        summary: "skip to the next turn of the innermost loop",
        max_args: Some(0),
        handler: run_continue,
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
        summary: "bring a job to the foreground and wait for it",
        max_args: Some(1),
        handler: run_fg,
    },
    CommandSpec {
        name: "bg",
        usage: "bg [%job]",
        summary: "continue a stopped job in the background",
        max_args: Some(1),
        handler: run_bg,
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
    // Here-documents become here-strings before anything is parsed.
    let rewritten = match heredoc::rewrite(input) {
        Ok(rewritten) => rewritten,
        Err(error) => return syntax_error(error, state),
    };
    let input = rewritten.as_str();
    let segments = match split_chain(input) {
        Ok(segments) => segments,
        Err(error) => return syntax_error(error, state),
    };
    // A syntax error anywhere, in a compound command too, rejects the whole
    // input before anything runs.
    let nodes = match flow::parse(segments) {
        Ok(nodes) => nodes,
        Err(error) => return syntax_error(error, state),
    };
    flow::run_nodes(&nodes, state)
}

/// Whether `input` opens a `for`, `while`, `until` or `if` that is not closed:
/// the front ends read the next line before running it.
pub(crate) fn input_is_incomplete(input: &str) -> bool {
    // A here-document is open until its delimiter line; once closed, the blocks are
    // counted on the text with the bodies taken out, so a body cannot fool them.
    if heredoc::is_open(input) {
        return true;
    }
    match heredoc::rewrite(input) {
        Ok(rewritten) => flow::is_incomplete(&rewritten),
        Err(_) => false,
    }
}

/// Whether a here-document of `text` is still waiting for its delimiter line.
pub(super) fn heredoc_is_open(text: &str) -> bool {
    heredoc::is_open(text)
}

/// The words after `in` of a `for`, expanded like the arguments of a command.
fn expand_words(text: &str, state: &mut ShellState) -> Result<Vec<String>, String> {
    let braced = braces::expand(text)?;
    let expanded = substitution::expand(&braced, state)?;
    parse_args(&expanded, &|name| state.variable(name))
}

/// The word a `case` looks at: expanded like an argument, but never matched against
/// file names and never split.
fn expand_case_word(text: &str, state: &mut ShellState) -> Result<String, String> {
    let expanded = substitution::expand(text, state)?;
    let words = crate::app::parser::parse_words(&expanded, &|name| state.variable(name))?;
    Ok(words
        .into_iter()
        .map(|word| word.text)
        .collect::<Vec<_>>()
        .join(" "))
}

/// A pattern of a `case` arm: what was typed as a wildcard matches, and what was
/// quoted, escaped or comes from a variable stands for itself.
fn expand_case_pattern(text: &str, state: &mut ShellState) -> Result<String, String> {
    let expanded = substitution::expand(text, state)?;
    let words = crate::app::parser::parse_words(&expanded, &|name| state.variable(name))?;
    match words.as_slice() {
        [] => Ok(String::new()),
        [word] => Ok(word
            .pattern
            .clone()
            .unwrap_or_else(|| crate::utils::glob::escape(&word.text))),
        _ => Err(format!("case: invalid pattern '{}'", text.trim())),
    }
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
    // `[[ expression ]]` reads its own words: no braces, no splitting, no patterns.
    if let Some(body) = double_bracket::body(text) {
        return match body {
            Ok(body) => condition(double_bracket::evaluate(body, state)),
            Err(error) => CommandResult::Error(error),
        };
    }
    // Braces first, then command substitutions, which run now: each rewrites the
    // text, which is then split again.
    let braced = match braces::expand(text) {
        Ok(braced) => braced,
        Err(error) => return CommandResult::Error(error),
    };
    if braced.contains("$(") || braced.contains('`') || braced != text {
        let expanded = match substitution::expand(&braced, state) {
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
    pipeline::run(stages, state, background.then_some(text), None)
}

/// How a pipeline stage runs.
enum StageKind {
    /// Inside the shell: built-in commands, plugin commands, assignments.
    InProcess,
    /// As a program, with its streams connected to the neighbours.
    Program { program: String, args: Vec<String> },
    /// The native Git, which has its own environment (the bundled runtime).
    Git { args: Vec<String> },
    /// A `for`, `while`, `if`, `case` or `{ }` that is a stage of a pipeline or has
    /// redirections: it runs inside the shell, and its output is collected.
    Compound,
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
    // A function takes the place of any command of the same name.
    if state.functions.contains_key(name) {
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

    if let Some(body) = state.functions.get(command_name).cloned() {
        return functions::call(command_name, &body, &args[1..], state);
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

fn run_pwd(args: &[String], _state: &mut ShellState) -> CommandResult {
    match args.first().map(String::as_str) {
        None | Some("-L" | "-P" | "--") => map_prefixed_error("pwd", pwd::pwd()),
        Some(option) => CommandResult::Error(format!("pwd: invalid option: {option}")),
    }
}

fn run_cd(args: &[String], state: &mut ShellState) -> CommandResult {
    let to_previous = optional_arg(args).map(str::trim) == Some("-");
    match cd::cd(optional_arg(args), state.previous_dir.as_deref()) {
        Ok(previous_dir) => {
            if previous_dir.is_some() {
                state.previous_dir = previous_dir;
            }
            cd::record_change(state.previous_dir.as_deref());
            // `cd -` says where it went, as other shells do.
            if to_previous && let Ok(now) = std::env::current_dir() {
                return map_prefixed_error("cd", render::stdout_line(now.display()));
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

/// One message for every failure of a command that handles several targets.
fn failures<E: std::fmt::Display>(errors: Vec<E>) -> CommandResult {
    if errors.is_empty() {
        return CommandResult::Continue;
    }
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    CommandResult::Error(messages.join("\n"))
}

fn run_mkdir(args: &[String], _state: &mut ShellState) -> CommandResult {
    failures(mkdir::mkdir_all(args))
}

fn run_touch(args: &[String], _state: &mut ShellState) -> CommandResult {
    failures(touch::touch_all(args))
}

fn run_cat(args: &[String], _state: &mut ShellState) -> CommandResult {
    let (content, errors) = cat::cat_all(args);
    if !content.is_empty()
        && let Err(error) = render::stdout(content)
    {
        return CommandResult::Error(format!("cat: {error}"));
    }
    failures(errors)
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
    // `unset -f name`: forget functions instead of variables.
    if args.first().map(String::as_str) == Some("-f") {
        for name in &args[1..] {
            state.functions.remove(name.as_str());
        }
        return CommandResult::Continue;
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

fn run_history(args: &[String], state: &mut ShellState) -> CommandResult {
    let count = match args.first() {
        None => state.history.len(),
        Some(text) => match text.parse::<usize>() {
            Ok(count) => count,
            Err(_) => {
                return CommandResult::Error(format!("history: not a number: {text}"));
            }
        },
    };
    let first = state.history.len().saturating_sub(count);
    for (index, line) in state.history.iter().enumerate().skip(first) {
        if let Err(error) = render::stdout_line(format!("{:>5}  {line}", index + 1)) {
            return CommandResult::Error(format!("history: {error}"));
        }
    }
    CommandResult::Continue
}

fn run_source(args: &[String], state: &mut ShellState) -> CommandResult {
    let Some((path, arguments)) = args.split_first() else {
        return CommandResult::Error("source: expected a file".to_string());
    };
    // The arguments are `$1`, `$2`, ... for the length of the file.
    state.frames.push(functions::Frame::for_file(arguments));
    let result = scripts::run_file("source", path, state, false);
    state.frames.pop();
    result
}

fn run_read(args: &[String], state: &mut ShellState) -> CommandResult {
    let request = match read_command::parse(args) {
        Ok(request) => request,
        Err(message) => return CommandResult::Error(message),
    };
    // A prompt is for someone typing: a pipe or a redirection does not see it.
    if let Some(prompt) = &request.prompt
        && !crate::utils::pipe_input::is_provided()
    {
        let _ = render::stderr(prompt);
    }
    let line = read_command::next_line(&mut std::io::stdin().lock());
    let values = match &line {
        Some(line) => read_command::split_fields(line, &request.names),
        None => request
            .names
            .iter()
            .map(|name| (name.clone(), String::new()))
            .collect(),
    };
    for (name, value) in values {
        assign_variable(&name, &value, state);
    }
    match line {
        Some(_) => CommandResult::Continue,
        // The end of the input is a failure without a message, so that
        // `while read` style tests can stop on it.
        None => CommandResult::Error(String::new()),
    }
}

/// What running `name` would do, in the order the shell looks it up.
fn describe_name(name: &str, state: &ShellState) -> Option<String> {
    if flow::is_keyword(name) {
        return Some(format!("{name} is a shell keyword"));
    }
    if state.functions.contains_key(name) {
        return Some(format!("{name} is a function"));
    }
    if find_command(name).is_some() {
        return Some(format!("{name} is a shell builtin"));
    }
    if let Some(command) = state.user_commands.get(name) {
        let words = std::iter::once(command.target.as_str())
            .chain(command.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        return Some(match command.kind {
            UserCommandKind::Builtin => format!("{name} is an alias for '{words}'"),
            UserCommandKind::Process => format!("{name} is a custom command running '{words}'"),
        });
    }
    if state.plugin_commands.get(name).is_some() {
        return Some(format!("{name} is a plugin command"));
    }
    external::locate(name).map(|path| format!("{name} is {}", path.display()))
}

/// The process id of the parent of the shell, empty where it is not known.
fn parent_process_id() -> String {
    #[cfg(unix)]
    {
        std::os::unix::process::parent_id().to_string()
    }
    #[cfg(not(unix))]
    {
        String::new()
    }
}

/// `$RANDOM`: xorshift, seeded from the clock and the process id the first time.
fn next_random() -> u32 {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    STATE.with(|cell| {
        let mut value = cell.get();
        if value == 0 {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos() as u64);
            value = nanos ^ (u64::from(std::process::id()) << 32) | 1;
        }
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        cell.set(value);
        ((value >> 33) % 32768) as u32
    })
}

/// `command -v name...` prints what each name runs, `command -V name...` says it like
/// `type`, and `command name args...` runs the command without the function of that name.
fn run_command(args: &[String], state: &mut ShellState) -> CommandResult {
    match args.first().map(String::as_str) {
        None => CommandResult::Error("command: usage: command [-v|-V] name [args...]".into()),
        Some("-V") => run_type(&args[1..], state),
        Some("-v") => {
            let mut missing = false;
            for name in &args[1..] {
                let shown = if flow::is_keyword(name)
                    || state.functions.contains_key(name)
                    || find_command(name).is_some()
                    || state.user_commands.get(name).is_some()
                {
                    Some(name.clone())
                } else {
                    external::locate(name).map(|path| path.display().to_string())
                };
                match shown {
                    Some(line) => {
                        let _ = render::stdout_line(line);
                    }
                    None => missing = true,
                }
            }
            if missing || args.len() == 1 {
                CommandResult::Error(String::new())
            } else {
                CommandResult::Continue
            }
        }
        Some(name) => {
            // A function does not shadow the command that is asked for by name.
            let shadowed = state.functions.remove(name);
            let result = execute(args, state);
            if let Some(function) = shadowed {
                state.functions.entry(name.to_string()).or_insert(function);
            }
            result
        }
    }
}

/// `trap 'command' EXIT` runs the command when the shell ends (`exit`, end of input, `Ctrl+D`);
/// `trap - EXIT` removes it, and `trap` alone shows it. Signal traps are not available.
fn run_trap(args: &[String], state: &mut ShellState) -> CommandResult {
    let is_exit = |name: &str| matches!(name, "EXIT" | "0");
    match args {
        [] => {
            if let Some(command) = &state.exit_trap {
                let quoted = command.replace('\'', "'\\''");
                return map_prefixed_error(
                    "trap",
                    render::stdout_line(format!("trap -- '{quoted}' EXIT")),
                );
            }
            CommandResult::Continue
        }
        [dash, names @ ..] if dash == "-" && !names.is_empty() => {
            for name in names {
                if !is_exit(name) {
                    return CommandResult::Error(trap_unsupported(name));
                }
            }
            state.exit_trap = None;
            CommandResult::Continue
        }
        [command, names @ ..] if !names.is_empty() => {
            for name in names {
                if !is_exit(name) {
                    return CommandResult::Error(trap_unsupported(name));
                }
            }
            state.exit_trap = Some(command.clone()).filter(|command| !command.trim().is_empty());
            CommandResult::Continue
        }
        _ => CommandResult::Error("trap: usage: trap [command] EXIT | trap - EXIT".to_string()),
    }
}

fn trap_unsupported(name: &str) -> String {
    format!("trap: {name}: only EXIT is supported (no signal traps)")
}

/// `exec command args...` runs the command and leaves the shell with its status; without
/// a command it does nothing. The shell is not replaced by the program.
fn run_exec(args: &[String], state: &mut ShellState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Continue;
    }
    let result = execute(args, state);
    if let CommandResult::Error(message) = &result
        && !message.is_empty()
    {
        let _ = render::stderr_line(message);
    }
    state.record_status(matches!(result, CommandResult::Error(_)));
    state.exit_code = Some(status_byte(i64::from(state.last_status)));
    CommandResult::Exit
}

fn run_type(args: &[String], state: &mut ShellState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("type: usage: type <name>...".to_string());
    }
    let mut missing = Vec::new();
    for name in args {
        match describe_name(name, state) {
            Some(line) => {
                let _ = render::stdout_line(line);
            }
            None => missing.push(format!("type: {name}: not found")),
        }
    }
    if missing.is_empty() {
        CommandResult::Continue
    } else {
        CommandResult::Error(missing.join("\n"))
    }
}

/// A condition: success when true, a silent failure (status 1) when false, and a
/// message with status 2 when the expression is wrong.
fn condition(outcome: test_command::Outcome) -> CommandResult {
    match outcome {
        Ok(true) => CommandResult::Continue,
        Ok(false) => CommandResult::Error(String::new()),
        Err(message) => {
            external::note_exit_code(2);
            CommandResult::Error(message)
        }
    }
}

fn run_test(args: &[String], _state: &mut ShellState) -> CommandResult {
    condition(test_command::evaluate(args, false))
}

fn run_bracket(args: &[String], _state: &mut ShellState) -> CommandResult {
    condition(test_command::evaluate(args, true))
}

fn run_true(_args: &[String], _state: &mut ShellState) -> CommandResult {
    CommandResult::Continue
}

/// An empty message: the command failed, and there is nothing to report.
fn run_false(_args: &[String], _state: &mut ShellState) -> CommandResult {
    CommandResult::Error(String::new())
}

fn loop_control(name: &str, signal: flow::LoopSignal, state: &mut ShellState) -> CommandResult {
    if state.loop_depth == 0 {
        return CommandResult::Error(format!("{name}: only meaningful in a loop"));
    }
    state.loop_signal = Some(signal);
    CommandResult::Continue
}

fn run_break(_args: &[String], state: &mut ShellState) -> CommandResult {
    loop_control("break", flow::LoopSignal::Break, state)
}

fn run_continue(_args: &[String], state: &mut ShellState) -> CommandResult {
    loop_control("continue", flow::LoopSignal::Continue, state)
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
    let announce = |text: &str| {
        let _ = render::stdout_line(text);
        // The job has the terminal while it runs: show the command first.
        render::flush_captured();
    };
    match state.jobs.foreground(optional_arg(args), announce) {
        Ok(jobs::Foreground::Done(status)) => job_outcome("fg", status),
        Ok(jobs::Foreground::Stopped(notice)) => {
            let _ = render::stderr_line(notice);
            external::note_exit_code(crate::utils::job_control::STOPPED_STATUS);
            CommandResult::Error(String::new())
        }
        Err(error) => CommandResult::Error(format!("fg: {error}")),
    }
}

fn run_bg(args: &[String], state: &mut ShellState) -> CommandResult {
    match state.jobs.background(optional_arg(args)) {
        Ok(line) => map_prefixed_error("bg", render::stdout_line(line)),
        Err(error) => CommandResult::Error(format!("bg: {error}")),
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
    // Stopped jobs would be left behind: say so once, and leave on the next `exit`.
    if state.jobs.has_stopped() && !state.exit_warned {
        state.exit_warned = true;
        return CommandResult::Error("There are stopped jobs.".to_string());
    }
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
        // `pwd` and the expected value must see the same directory: other tests
        // change it while they hold this lock.
        let _guard = crate::test_support::cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    fn an_alias_can_be_defined_as_name_equals_value() {
        let mut state = test_state();
        state
            .variables
            .insert("TR_ALIAS_VAR".to_string(), "expanded".to_string());
        let mut define = |arguments: &[&str]| {
            let mut line = vec!["alias"];
            line.extend_from_slice(arguments);
            render::capture_output(|| execute(&string_args(&line), &mut state)).0
        };

        assert!(matches!(define(&["ll=ls -l"]), CommandResult::Continue));
        assert!(matches!(
            define(&["say=echo 'two words' \"q r\""]),
            CommandResult::Continue
        ));
        assert!(matches!(
            define(&["first=echo $TR_ALIAS_VAR", "second=pwd"]),
            CommandResult::Continue
        ));
        assert!(matches!(
            define(&["pat=echo *.rs"]),
            CommandResult::Continue
        ));

        let get = |name: &str| {
            state
                .user_commands
                .get(name)
                .map(|alias| (alias.target.clone(), alias.args.clone()))
        };
        assert_eq!(get("ll"), Some(("ls".to_string(), string_args(&["-l"]))));
        assert_eq!(
            get("say"),
            Some(("echo".to_string(), string_args(&["two words", "q r"])))
        );
        // Variables are expanded when the alias is defined; patterns are not.
        assert_eq!(
            get("first"),
            Some(("echo".to_string(), string_args(&["expanded"])))
        );
        assert_eq!(get("second"), Some(("pwd".to_string(), Vec::new())));
        assert_eq!(
            get("pat"),
            Some(("echo".to_string(), string_args(&["*.rs"])))
        );
    }

    #[test]
    fn a_bad_alias_definition_is_reported_and_stops_the_list() {
        let mut state = test_state();
        let mut run = |arguments: &[&str]| {
            let mut line = vec!["alias"];
            line.extend_from_slice(arguments);
            match render::capture_output(|| execute(&string_args(&line), &mut state)).0 {
                CommandResult::Error(message) => message,
                _ => String::new(),
            }
        };

        assert!(run(&["empty="]).contains("the definition is empty"));
        assert!(run(&["ok=pwd", "oops"]).contains("oops: expected name=value"));
        assert!(run(&["BadName=pwd"]).contains("alias"));
        assert!(run(&["quote=echo 'open"]).contains("missing closing quote"));
        // An `=` inside the command of the original form is not a definition.
        assert_eq!(run(&["envy", "env", "FOO=bar"]), "");
        assert!(run(&["only"]).contains("expected"));
        // A definition needs a name: this is not one, so the usage is shown.
        assert!(run(&["=x"]).contains("or <name>=<command>"));
        // The first definition before the mistake was kept, the one after was not.
        assert!(state.user_commands.get("ok").is_some());
        assert!(state.user_commands.get("envy").is_some());
    }

    #[test]
    fn history_lists_numbered_lines_and_the_last_ones() {
        let mut state = test_state();
        state.set_history(
            ["one", "two", "three"]
                .iter()
                .map(ToString::to_string)
                .collect(),
        );

        let (result, output) =
            render::capture_output(|| execute(&string_args(&["history"]), &mut state));
        assert!(matches!(result, CommandResult::Continue));
        assert_eq!(output.stdout, "    1  one\n    2  two\n    3  three\n");

        let (_, output) =
            render::capture_output(|| execute(&string_args(&["history", "2"]), &mut state));
        assert_eq!(output.stdout, "    2  two\n    3  three\n");

        let (_, output) =
            render::capture_output(|| execute(&string_args(&["history", "99"]), &mut state));
        assert_eq!(output.stdout.lines().count(), 3);

        let (result, _) =
            render::capture_output(|| execute(&string_args(&["history", "x"]), &mut state));
        assert!(
            matches!(result, CommandResult::Error(message) if message.contains("not a number"))
        );

        state.record_history("four");
        assert_eq!(state.history().last().map(String::as_str), Some("four"));
    }

    #[test]
    fn the_body_of_a_here_document_does_not_open_or_close_blocks() {
        // Words that start blocks or end them are text inside a body.
        for body in [
            "for i in 1",
            "if true",
            "while x",
            "case a in",
            "{",
            "f() {",
            "done",
            "fi",
            "esac",
            "}",
        ] {
            let text = format!("cat <<EOF\n{body}\nEOF");
            assert!(!input_is_incomplete(&text), "{body}");
        }
        // The line that follows a closed here-document counts as usual.
        assert!(input_is_incomplete("cat <<EOF\nx\nEOF\nfor i in 1; do"));
        assert!(!input_is_incomplete(
            "cat <<EOF\nx\nEOF\nfor i in 1; do echo; done"
        ));
        // Still open without the delimiter, whatever the body holds.
        assert!(input_is_incomplete("cat <<EOF\ndone"));
    }

    #[test]
    fn type_says_what_a_name_runs() {
        let mut state = test_state();
        let _ = render::capture_output(|| {
            execute(&string_args(&["alias", "where", "pwd"]), &mut state)
        });
        let say = |name: &str, state: &mut ShellState| {
            describe_name(name, state).unwrap_or_else(|| "none".to_string())
        };

        assert_eq!(say("for", &mut state), "for is a shell keyword");
        assert_eq!(say("echo", &mut state), "echo is a shell builtin");
        assert_eq!(say("where", &mut state), "where is an alias for 'pwd'");
        assert_eq!(say("terminal-rust-no-such-program", &mut state), "none");
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

    /// Random text made of the characters the parsers care about must never make
    /// one of them panic: a shell that dies on a strange line is a bad shell. A
    /// fixed seed keeps a failure reproducible.
    #[test]
    fn the_parsers_never_panic_on_random_input() {
        const PIECES: [&str; 40] = [
            "a", "b", "x", "1", "2", " ", "  ", "\n", "\t", "$", "$(", "${", "(", ")", "{", "}",
            ",", "..", ";", "&", "&&", "|", "||", "<", ">", ">>", "2>&1", "'", "\"", "\\", "*",
            "?", "[", "]", "~", "=", "for ", "do ", "done", "if ",
        ];
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..150_000 {
            let length = 1 + (next() % 20) as usize;
            let text: String = (0..length)
                .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                .collect();

            let outcome = std::panic::catch_unwind(|| {
                let _ = braces::expand(&text);
                let _ = flow::is_incomplete(&text);
                let _ = crate::app::parser::parse_words(&text, &|_| None);
                let _ = split_stages(&text);
                if let Ok(segments) = split_chain(&text) {
                    let _ = flow::parse(segments);
                }
            });
            assert!(outcome.is_ok(), "a parser panicked on {text:?}");
        }
    }

    /// Run random lines made of harmless commands. Nothing may panic or hang,
    /// whatever the mix of quotes, loops, substitutions and redirections.
    #[test]
    fn random_lines_of_harmless_commands_never_panic_or_hang() {
        use std::sync::mpsc::{self, RecvTimeoutError};
        use std::time::Duration;

        const PIECES: [&str; 57] = [
            "echo ",
            "true",
            "false",
            "test ",
            "[ ",
            " ]",
            "break",
            "continue",
            "a",
            "b",
            "1",
            " ",
            " ",
            "; ",
            " && ",
            " || ",
            " | ",
            " > /dev/null ",
            " >> /dev/null ",
            " 2>&1 ",
            "$(",
            ")",
            "{a,b}",
            "{1..3}",
            "'",
            "\"",
            "$x",
            "*",
            "for x in 1 2; do ",
            "; done",
            "if true; then ",
            "; fi",
            "if false; then ",
            "; else ",
            "\n",
            "x=",
            "$((",
            "))",
            "case x in ",
            "a) ",
            ";; ",
            "esac",
            "f() { ",
            "}",
            "; return ",
            "$1",
            "\"$@\"",
            "local x=1",
            "f ",
            "<<EOF\n",
            "\nEOF\n",
            " <<< x",
            " < /dev/null",
            " | while read x; do ",
            "done",
            "{ ",
            " }",
        ];
        // The lines never create a file (their redirections go to `/dev/null`) and
        // never change directory, so nothing here touches the process-wide state
        // that other tests read: no fixture directory and no lock.
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut state = test_state();
            let mut seed = 0xD1B5_4A32_D192_ED03_u64;
            let mut next = move || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed
            };
            for _ in 0..6_000 {
                let length = 1 + (next() % 14) as usize;
                let line: String = (0..length)
                    .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                    .collect();
                let _ = sender.send(line.clone());
                // The output goes nowhere: it is captured and dropped.
                let _ = render::capture_output(|| run_line(&line, &mut state));
            }
        });

        let mut last = String::new();
        let mut hung = false;
        loop {
            match receiver.recv_timeout(Duration::from_secs(20)) {
                Ok(line) => last = line,
                Err(RecvTimeoutError::Timeout) => {
                    hung = true;
                    break;
                }
                // The sender is gone because the worker ended or panicked: it
                // may not be finished a moment later, so wait for it.
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        let result = (!hung).then(|| worker.join());
        assert!(!hung, "the shell hung on {last:?}");
        assert!(
            result.is_some_and(|result| result.is_ok()),
            "the shell panicked on {last:?}"
        );
    }
}
