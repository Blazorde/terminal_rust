//! Connect the built-ins to the system programs that take over from them (see
//! `commands::system_fallback`).

use std::path::PathBuf;

use super::{CommandResult, ShellState, StageKind, find_command, finish_external, pipeline};
use crate::commands::system_fallback::{self, Decision};
use crate::commands::{external, rm};
use crate::config::UserCommandKind;
use crate::render;
use crate::utils::pipe_input;

fn decide(name: &str, args: &[String]) -> Decision {
    system_fallback::decide(name, args, cfg!(windows), external::locate_absolute)
}

/// Run `name args` in the place of the built-in, or `None` when the built-in does it.
pub(super) fn dispatch(
    name: &str,
    args: &[String],
    state: &mut ShellState,
) -> Option<CommandResult> {
    match decide(name, args) {
        Decision::Builtin => None,
        Decision::Reject(message) => Some(CommandResult::Error(message)),
        Decision::System(program) => Some(run(name, args, program, state)),
    }
}

fn run(name: &str, args: &[String], program: PathBuf, state: &mut ShellState) -> CommandResult {
    let kind = StageKind::System {
        name: name.to_string(),
        program: program.clone(),
        args: args.to_vec(),
    };
    // Only the pipeline code collects output and passes the input of a pipeline.
    if render::programs_captured() || reads_input(name) {
        return pipeline::run_program(name, args, kind, state);
    }
    match guard(&kind) {
        Ok(true) => finish_external(name, external::run_system(&program, name, args)),
        Ok(false) => CommandResult::Continue,
        Err(message) => CommandResult::Error(message),
    }
}

/// The checks to make right before a stage starts: `Ok(false)` when the user
/// declined, an error when the command is refused.
pub(super) fn guard(kind: &StageKind) -> Result<bool, String> {
    match kind {
        StageKind::System { name, args, .. } if name == "rm" => {
            rm::guard_system(args).map_err(|error| error.to_string())
        }
        _ => Ok(true),
    }
}

fn reads_input(name: &str) -> bool {
    system_fallback::handled(name).is_some_and(|handled| handled.reads_input)
}

/// What the shell holds of the input of a pipeline, for a program that is the first
/// to run and would otherwise read the terminal. Left alone for the programs that
/// do not read it, so that a `while read` loop keeps its lines.
pub(super) fn held_input(kind: &StageKind) -> Option<Vec<u8>> {
    let StageKind::System { name, args, .. } = kind else {
        return None;
    };
    let mut operands = system_fallback::operands(args).peekable();
    let wants = reads_input(name) && (operands.peek().is_none() || operands.any(|arg| arg == "-"));
    if wants { pipe_input::take() } else { None }
}

/// The stage of a command that a system program takes over, also seen through an
/// alias of a built-in and through `command NAME`. `None` leaves it in the shell.
pub(super) fn stage(name: &str, args: &[String], state: &ShellState) -> Option<StageKind> {
    if name == "command" {
        let (target, rest) = args.split_first()?;
        // A function does not count here: `command` runs without it.
        return (!target.starts_with('-'))
            .then(|| unshadowed_stage(target, rest, state))
            .flatten();
    }
    unshadowed_stage(name, args, state)
}

fn unshadowed_stage(name: &str, args: &[String], state: &ShellState) -> Option<StageKind> {
    if name == "command" {
        return stage(name, args, state);
    }
    if find_command(name).is_some() {
        return match decide(name, args) {
            Decision::System(program) => Some(StageKind::System {
                name: name.to_string(),
                program,
                args: args.to_vec(),
            }),
            Decision::Builtin | Decision::Reject(_) => None,
        };
    }
    let alias = state.user_commands.get(name)?;
    if alias.kind != UserCommandKind::Builtin
        || (!alias.append_args && !args.is_empty())
        || state.functions.contains_key(&alias.target)
    {
        return None;
    }
    let mut words = alias.args.clone();
    words.extend_from_slice(args);
    stage(&alias.target, &words, state)
}
