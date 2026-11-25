//! Run the stages of a pipeline with their redirections (see ADR-0011).
//!
//! A stage is a program, started with its streams connected, or an in-process
//! command, whose captured text feeds the next stage.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::thread::{self, JoinHandle};

use super::{
    CommandError, CommandResult, ShellState, StageKind, classify, execute, find_command, flow,
    split_env_prefix,
};
use crate::app::parser::{Redirect, Stage, parse_args, parse_with};
use crate::commands::external::{self, RunError};
use crate::git;
use crate::render;
use crate::utils::{job_control, pipe_input, shell_env};

/// The text of the stage that stands for the compound command of the pipeline. It
/// holds the redirections that were written after the closing keyword.
pub(super) const COMPOUND_STAGE: &str = "\u{1}compound";

/// Where one output stream of a stage goes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sink {
    /// The pipe to the next stage, or the terminal for the last one.
    Default,
    /// The file opened for the redirection with this index.
    File(usize),
    /// The default destination of the *other* output stream (`2>&1`, `>&2`).
    OtherDefault,
}

struct Target {
    path: PathBuf,
    append: bool,
}

/// A stage with its words and redirections expanded, ready to run.
struct Plan {
    args: Vec<String>,
    env: Vec<(String, String)>,
    kind: StageKind,
    stdin: Option<PathBuf>,
    /// The text of a here-string, which is the input instead of `stdin`.
    here_text: Option<Vec<u8>>,
    out: Sink,
    err: Sink,
    files: Vec<Target>,
}

/// What a stage reads.
enum Input {
    /// The terminal, for the first stage.
    Inherit,
    /// Nothing: the previous stage left no output.
    Empty,
    /// The pipe from the previous program.
    Pipe(io::PipeReader),
    /// The text an in-process command produced.
    Text(Vec<u8>),
}

struct Opened {
    stdin: Option<File>,
    files: Vec<File>,
}

/// `background` is the text of the command when the pipeline runs as a job, and
/// `compound` the command that the stage [`COMPOUND_STAGE`] stands for, if any.
pub(super) fn run(
    stages: &[Stage],
    state: &mut ShellState,
    background: Option<&str>,
    compound: Option<&flow::Node>,
) -> CommandResult {
    // Everything is expanded before anything runs, so a bad line changes nothing.
    let mut plans = Vec::with_capacity(stages.len());
    for stage in stages {
        match plan(stage, state) {
            Ok(plan) => plans.push(plan),
            Err(error) => return CommandResult::Error(error),
        }
    }

    execute_plans(plans, state, background, compound, &|| {
        pipeline_text(stages)
    })
}

/// Run one program the way a stage of a pipeline does, so that what it writes is
/// collected when the shell collects the output of programs. `kind` says what to
/// start; `name` and `args` are what was typed.
pub(super) fn run_program(
    name: &str,
    args: &[String],
    kind: StageKind,
    state: &mut ShellState,
) -> CommandResult {
    let mut words = vec![name.to_string()];
    words.extend_from_slice(args);
    let text = words.join(" ");
    let plan = Plan {
        args: words,
        env: Vec::new(),
        kind,
        stdin: None,
        here_text: None,
        out: Sink::Default,
        err: Sink::Default,
        files: Vec::new(),
    };
    execute_plans(vec![plan], state, None, None, &|| text.clone())
}

/// Start the expanded stages and wait for them. `title` names the job if the
/// pipeline is stopped.
fn execute_plans(
    plans: Vec<Plan>,
    state: &mut ShellState,
    background: Option<&str>,
    compound: Option<&flow::Node>,
    title: &dyn Fn() -> String,
) -> CommandResult {
    let count = plans.len();
    // A background program must not compete with the shell for the terminal.
    let mut input = if background.is_some() {
        Input::Empty
    } else {
        Input::Inherit
    };
    let mut children: Vec<(String, Child)> = Vec::new();
    // The process group of the programs: the first one leads it, the others join it.
    let mut group: Option<i32> = None;
    let mut feeders: Vec<JoinHandle<()>> = Vec::new();
    // In a command substitution, what the last program writes is collected.
    let capturing = background.is_none() && render::programs_captured();
    let mut collector: Option<JoinHandle<Vec<u8>>> = None;
    // The outcome of the last stage when it is not a program that was started.
    let mut last_result: Option<CommandResult> = None;
    let mut exit_requested = false;

    for (index, plan) in plans.into_iter().enumerate() {
        let is_last = index + 1 == count;
        let mut previous = std::mem::replace(&mut input, Input::Empty);
        // A here-string is the whole input of its stage.
        if let Some(text) = plan.here_text.clone() {
            previous = Input::Text(text);
        }
        let name = plan.args.first().cloned().unwrap_or_default();

        let mut opened = match open(&plan) {
            Ok(opened) => opened,
            Err(message) => {
                if is_last {
                    last_result = Some(CommandResult::Error(message));
                } else {
                    let _ = render::stderr_line(message);
                }
                continue;
            }
        };

        match &plan.kind {
            StageKind::InProcess | StageKind::Compound => {
                let stdin = read_input(opened.stdin.take(), previous, index == 0);
                let (result, next_input) =
                    run_in_process(&plan, opened, stdin, is_last, state, compound);
                exit_requested |= result == CommandResult::Exit;
                input = next_input;
                if is_last {
                    last_result = Some(result);
                } else if let CommandResult::Error(message) = result
                    && !message.is_empty()
                {
                    let _ = render::stderr_line(message);
                }
            }
            StageKind::Program { .. } | StageKind::Git { .. } => {
                match start_program(
                    &plan,
                    opened,
                    previous,
                    is_last,
                    group,
                    background.is_none(),
                ) {
                    Ok(started) => {
                        if group.is_none() && job_control::enabled() {
                            group = Some(job_control::group_of(&started.child));
                        }
                        input = started.next_input;
                        if is_last
                            && capturing
                            && let Input::Pipe(mut reader) =
                                std::mem::replace(&mut input, Input::Empty)
                        {
                            collector = Some(thread::spawn(move || {
                                let mut bytes = Vec::new();
                                let _ = reader.read_to_end(&mut bytes);
                                bytes
                            }));
                        }
                        feeders.extend(started.feeder);
                        children.push((name.clone(), started.child));
                        if is_last {
                            last_result = None;
                        }
                    }
                    Err(message) => {
                        if is_last {
                            last_result = Some(CommandResult::Error(message));
                        } else {
                            let _ = render::stderr_line(message);
                        }
                    }
                }
            }
        }
    }

    // A background pipeline is not waited for: its programs become a job.
    if let Some(text) = background {
        if !children.is_empty() {
            let programs = children.into_iter().map(|(_, child)| child).collect();
            let (number, pid) = state.jobs.add(text, programs, group);
            state.last_background = Some(pid);
            let _ = render::stderr_line(format!("[{number}] {pid}"));
        }
        if exit_requested {
            return CommandResult::Exit;
        }
        return last_result.unwrap_or(CommandResult::Continue);
    }

    // Wait for the programs. The status of the pipeline is the one of its last
    // stage: the last child when the last stage is a program.
    let mut program_result = None;
    let mut last_code = None;
    let started = children.len();
    let (names, programs): (Vec<String>, Vec<Child>) = children.into_iter().unzip();
    let statuses = match job_control::wait_foreground(programs, None) {
        job_control::Waited::Done(statuses) => statuses,
        // `Ctrl+Z`: the pipeline is a stopped job now, and says so itself. Whatever
        // still feeds or collects its streams is left to finish on its own.
        job_control::Waited::Stopped(stopped) => {
            job_control::set_aside(stopped, &title());
            external::note_exit_code(job_control::STOPPED_STATUS);
            return CommandResult::Error(String::new());
        }
    };
    for (position, (name, status)) in names.iter().zip(statuses).enumerate() {
        let outcome = if status.success() {
            CommandResult::Continue
        } else {
            if position + 1 == started {
                last_code = Some(external::exit_code(status));
            }
            CommandResult::Error(external::describe_exit(name, status))
        };
        if position + 1 == started {
            program_result = Some(outcome);
        }
    }
    for feeder in feeders {
        let _ = feeder.join();
    }
    // The programs are over, so their output is complete: add it to the capture.
    if let Some(collector) = collector
        && let Ok(bytes) = collector.join()
        && !bytes.is_empty()
    {
        let _ = render::stdout(String::from_utf8_lossy(&bytes));
    }

    if exit_requested {
        return CommandResult::Exit;
    }
    // `$?` is the code of the last program, when the last stage is one.
    if last_result.is_none()
        && let Some(code) = last_code
    {
        external::note_exit_code(code);
    }
    last_result
        .or(program_result)
        .unwrap_or(CommandResult::Continue)
}

/// The text that names a pipeline in the job table.
fn pipeline_text(stages: &[Stage]) -> String {
    stages
        .iter()
        .map(|stage| {
            if stage.text.trim() == COMPOUND_STAGE {
                "{ … }".to_string()
            } else {
                stage.text.trim().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn plan(stage: &Stage, state: &ShellState) -> Result<Plan, String> {
    let lookup = |name: &str| state.variable(name);
    let (env, args, kind) = if stage.text.trim() == COMPOUND_STAGE {
        (
            Vec::new(),
            vec![COMPOUND_STAGE.to_string()],
            StageKind::Compound,
        )
    } else {
        let words = parse_args(&stage.text, &lookup)?;
        if words.is_empty() {
            return Err("expected a command".to_string());
        }
        // `NAME=value command`: the variables apply to this stage only.
        let (env, rest) = split_env_prefix(&words);
        let args = rest.to_vec();
        let kind = classify(&args, state)?;
        (env, args, kind)
    };

    let mut stdin = None;
    let mut here_text = None;
    let mut out = Sink::Default;
    let mut err = Sink::Default;
    let mut files = Vec::new();
    let mut add_file = |target: &str, append: bool| -> Result<usize, String> {
        files.push(Target {
            path: expand_target(target, &lookup)?,
            append,
        });
        Ok(files.len() - 1)
    };

    for redirect in &stage.redirects {
        match redirect {
            // The last of `<` and `<<<` is the input.
            Redirect::Stdin(target) => {
                stdin = Some(expand_target(target, &lookup)?);
                here_text = None;
            }
            Redirect::HereString(word) => {
                let mut text = parse_with(word, &lookup)?.join(" ");
                text.push('\n');
                here_text = Some(text.into_bytes());
                stdin = None;
            }
            Redirect::Stdout { target, append } => out = Sink::File(add_file(target, *append)?),
            Redirect::Stderr { target, append } => err = Sink::File(add_file(target, *append)?),
            Redirect::StderrToStdout => {
                err = if out == Sink::Default {
                    Sink::OtherDefault
                } else {
                    out
                };
            }
            Redirect::StdoutToStderr => {
                out = if err == Sink::Default {
                    Sink::OtherDefault
                } else {
                    err
                };
            }
        }
    }
    // `>&2 2>&1` sends each stream to the other's default: leave both alone.
    if out == Sink::OtherDefault && err == Sink::OtherDefault {
        out = Sink::Default;
        err = Sink::Default;
    }

    Ok(Plan {
        args,
        env,
        kind,
        stdin,
        here_text,
        out,
        err,
        files,
    })
}

fn expand_target(target: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let mut words = parse_with(target, lookup)?;
    match (words.pop(), words.is_empty()) {
        (Some(word), true) if !word.is_empty() => Ok(PathBuf::from(word)),
        _ => Err(format!("{target}: ambiguous redirect")),
    }
}

fn open(plan: &Plan) -> Result<Opened, String> {
    let stdin = plan
        .stdin
        .as_deref()
        .map(|path| File::open(path).map_err(|error| describe(path, &error)))
        .transpose()?;
    let mut files = Vec::with_capacity(plan.files.len());
    for file in &plan.files {
        let mut options = OpenOptions::new();
        options.write(true).create(true);
        if file.append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        files.push(
            options
                .open(&file.path)
                .map_err(|error| describe(&file.path, &error))?,
        );
    }
    Ok(Opened { stdin, files })
}

fn describe(path: &Path, error: &io::Error) -> String {
    format!("cannot open '{}': {error}", path.display())
}

/// Run a command inside the shell and route its text.
fn run_in_process(
    plan: &Plan,
    mut opened: Opened,
    stdin: Option<Vec<u8>>,
    is_last: bool,
    state: &mut ShellState,
    compound: Option<&flow::Node>,
) -> (CommandResult, Input) {
    // The input is the stage's own for the duration of the command, and what the
    // command left unread is not for the next one. A first stage with no input of
    // its own shares the one of the loop it may be inside.
    let redirected = !is_last || plan.out != Sink::Default;
    let collects_programs = redirected && may_start_programs(&plan.args, state);
    let (result, captured) = pipe_input::scoped(stdin, || match (&plan.kind, compound) {
        // A compound command also collects the programs it starts.
        (StageKind::Compound, Some(node)) => {
            render::capture_substitution(|| flow::run_node(node, state))
        }
        _ => {
            let run = || shell_env::with_temporary(&plan.env, || execute(&plan.args, state));
            if collects_programs {
                render::capture_substitution(run)
            } else {
                render::capture_stage(run)
            }
        }
    });
    let mut stdout_text = captured.stdout;
    let mut stderr_text = captured.stderr;

    // The error of a command is what it writes on standard error. It stays
    // separate while it goes to the terminal, because the front end prints the
    // error of the last command itself.
    let error = match &result {
        CommandResult::Error(message) if !message.is_empty() => Some(message.clone()),
        _ => None,
    };
    let mut result = result;
    let mut error_line = None;
    if let Some(message) = error {
        match plan.err {
            Sink::Default => error_line = Some(message),
            Sink::File(_) | Sink::OtherDefault => {
                stderr_text.push_str(&message);
                stderr_text.push('\n');
                result = CommandResult::Error(String::new());
            }
        }
    }

    // A stream sent to the other one's default joins its text.
    match (plan.out, plan.err) {
        (Sink::OtherDefault, _) => {
            stderr_text.insert_str(0, &stdout_text);
            stdout_text.clear();
        }
        (_, Sink::OtherDefault) => {
            stdout_text.push_str(&stderr_text);
            stderr_text.clear();
        }
        _ => {}
    }

    let mut next_input = Input::Empty;
    match plan.out {
        Sink::File(index) => write_text(&mut opened.files[index], &stdout_text),
        Sink::Default | Sink::OtherDefault => {
            if is_last {
                if !stdout_text.is_empty() {
                    let _ = render::stdout(&stdout_text);
                }
            } else {
                next_input = Input::Text(stdout_text.into_bytes());
            }
        }
    }
    match plan.err {
        Sink::File(index) => write_text(&mut opened.files[index], &stderr_text),
        Sink::Default | Sink::OtherDefault => {
            if !stderr_text.is_empty() {
                let _ = render::stderr(&stderr_text);
            }
        }
    }
    if let Some(message) = error_line {
        if is_last {
            result = CommandResult::Error(message);
        } else {
            let _ = render::stderr_line(&message);
            result = CommandResult::Error(String::new());
        }
    }
    (result, next_input)
}

/// Whether an in-process command can start a program. The other built-ins are left
/// alone: collecting would also hide the questions they ask on the terminal.
fn may_start_programs(args: &[String], state: &ShellState) -> bool {
    let name = args.first().map(String::as_str).unwrap_or_default();
    state.functions.contains_key(name)
        || find_command(name).is_none()
        || matches!(
            name,
            "time" | "command" | "exec" | "eval" | "source" | "." | "kill"
        )
}

/// What an in-process command can read: a redirected file, or everything the
/// previous stage produced. `None` for the first stage, which has the terminal.
/// Reading a pipe to its end also lets the program that writes to it finish.
fn read_input(file: Option<File>, previous: Input, first: bool) -> Option<Vec<u8>> {
    let read_all = |mut source: Box<dyn Read>| {
        let mut bytes = Vec::new();
        let _ = source.read_to_end(&mut bytes);
        bytes
    };
    match (file, previous) {
        (Some(file), _) => Some(read_all(Box::new(file))),
        (None, Input::Pipe(reader)) => Some(read_all(Box::new(reader))),
        (None, Input::Text(bytes)) => Some(bytes),
        (None, Input::Empty) if !first => Some(Vec::new()),
        (None, Input::Empty | Input::Inherit) => None,
    }
}

fn write_text(file: &mut File, text: &str) {
    if !text.is_empty() {
        let _ = file.write_all(text.as_bytes());
    }
}

struct Started {
    child: Child,
    next_input: Input,
    feeder: Option<JoinHandle<()>>,
}

/// Start a program with its streams connected.
fn start_program(
    plan: &Plan,
    opened: Opened,
    previous: Input,
    is_last: bool,
    group: Option<i32>,
    foreground: bool,
) -> Result<Started, String> {
    let name = plan.args.first().map(String::as_str).unwrap_or_default();
    let capture_last = render::programs_captured();
    let mut command = match &plan.kind {
        StageKind::Program { program, args } => external::command(program, args),
        StageKind::Git { args } => git::native_command(args),
        StageKind::InProcess | StageKind::Compound => {
            return Err(format!("{name}: not a program"));
        }
    };
    // After the exports that `command` applied, so a prefix wins over them.
    command.envs(plan.env.iter().map(|(name, value)| (name, value)));

    let mut feed = None;
    match (opened.stdin, previous) {
        (Some(file), _) => {
            command.stdin(Stdio::from(file));
        }
        (None, Input::Inherit) => {}
        (None, Input::Empty) => {
            command.stdin(Stdio::null());
        }
        (None, Input::Pipe(reader)) => {
            command.stdin(Stdio::from(reader));
        }
        (None, Input::Text(bytes)) => {
            command.stdin(Stdio::piped());
            feed = Some(bytes);
        }
    }

    let mut next_reader = None;
    let mut pipe_writer = None;
    match plan.out {
        Sink::File(index) => {
            command.stdout(Stdio::from(clone(&opened.files[index])?));
        }
        Sink::OtherDefault => {
            command.stdout(Stdio::from(io::stderr()));
        }
        Sink::Default if !is_last || capture_last => {
            let (reader, writer) = io::pipe().map_err(|error| format!("{name}: {error}"))?;
            command.stdout(Stdio::from(
                writer
                    .try_clone()
                    .map_err(|error| format!("{name}: {error}"))?,
            ));
            next_reader = Some(reader);
            pipe_writer = Some(writer);
        }
        Sink::Default => {}
    }
    match plan.err {
        Sink::File(index) => {
            command.stderr(Stdio::from(clone(&opened.files[index])?));
        }
        Sink::OtherDefault => match (plan.out, pipe_writer.take()) {
            (_, Some(writer)) => {
                command.stderr(Stdio::from(writer));
            }
            (Sink::Default, None) => {
                command.stderr(Stdio::from(io::stdout()));
            }
            _ => {}
        },
        Sink::Default => {}
    }

    job_control::prepare(&mut command, group);
    let spawned = command.spawn();
    // The parent must not keep a copy of the write end, or the next stage would
    // never see the end of its input.
    drop(command);
    let mut child = spawned.map_err(|error| match external::start_error(name, &error) {
        RunError::NotFound => {
            if is_last {
                external::note_exit_code(external::COMMAND_NOT_FOUND);
            }
            CommandError::UnknownCommand(name.to_string()).to_string()
        }
        RunError::Failed(message) => message,
    })?;

    job_control::join_group(&child, group, foreground);

    let feeder = feed.and_then(|bytes| {
        child.stdin.take().map(|mut stdin| {
            thread::spawn(move || {
                // The program may stop reading early: that is not an error.
                let _ = stdin.write_all(&bytes);
            })
        })
    });
    let next_input = match next_reader {
        Some(reader) => Input::Pipe(reader),
        None => Input::Empty,
    };
    Ok(Started {
        child,
        next_input,
        feeder,
    })
}

fn clone(file: &File) -> Result<File, String> {
    file.try_clone()
        .map_err(|error| format!("cannot duplicate a file handle: {error}"))
}
