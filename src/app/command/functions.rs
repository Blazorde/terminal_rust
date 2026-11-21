//! Functions and positional parameters (see ADR-0021).
//!
//! A function is a parsed block kept by name. Calling it pushes a frame that
//! holds the arguments (`$1`, `$#`, `$@`) and the variables made `local`, runs the
//! block, then pops the frame and puts the locals back. `return` leaves the block
//! through a flag that every list and loop checks, like `break`.

use std::sync::Arc;

use super::{CommandResult, ShellState, flow};
use crate::commands::external;
use crate::utils::shell_env;

/// How many calls can be open at once: a function that calls itself without end
/// stops here with an error instead of exhausting the stack.
pub(super) const MAX_CALL_DEPTH: usize = 64;

/// The parsed block of a function.
pub(super) type Body = Arc<Vec<flow::Node>>;

/// One call of a function, or one `source` of a file with arguments.
pub(super) struct Frame {
    /// `$1`, `$2`, ... in order.
    pub(super) positional: Vec<String>,
    /// The variables made `local` here, with the value each had before.
    locals: Vec<(String, Option<String>)>,
    /// Whether this is a function call: only there are `return` and `local` allowed.
    function: bool,
}

impl Frame {
    /// A frame for a file run by `source`: it has arguments, but is no function.
    pub(super) fn for_file(arguments: &[String]) -> Self {
        Self {
            positional: arguments.to_vec(),
            locals: Vec::new(),
            function: false,
        }
    }
}

/// Run the function `name` with `arguments`.
pub(super) fn call(
    name: &str,
    body: &Body,
    arguments: &[String],
    state: &mut ShellState,
) -> CommandResult {
    if state.frames.len() >= MAX_CALL_DEPTH {
        external::note_exit_code(1);
        return CommandResult::Error(format!(
            "{name}: maximum function nesting depth ({MAX_CALL_DEPTH}) exceeded"
        ));
    }
    state.frames.push(Frame {
        positional: arguments.to_vec(),
        locals: Vec::new(),
        function: true,
    });
    let result = flow::run_nodes(body, state);
    if let Some(frame) = state.frames.pop() {
        restore_locals(frame, state);
    }
    let returned = state.returning.take();

    if result == CommandResult::Exit {
        return result;
    }
    match returned {
        Some(0) => CommandResult::Continue,
        Some(code) => {
            external::note_exit_code(code);
            CommandResult::Error(String::new())
        }
        None => {
            // The status of the last command is the one of the function: keep its
            // exact code when it was a program that failed.
            if matches!(result, CommandResult::Error(_)) && state.last_status != 0 {
                external::note_exit_code(state.last_status);
            }
            result
        }
    }
}

/// Put back the variables a frame made `local`, last one first.
fn restore_locals(frame: Frame, state: &mut ShellState) {
    for (name, previous) in frame.locals.into_iter().rev() {
        match previous {
            Some(value) => {
                state.variables.insert(name, value);
            }
            None => {
                state.variables.remove(&name);
            }
        }
    }
}

/// `return [status]`: leave the function that is running.
pub(super) fn run_return(arguments: &[String], state: &mut ShellState) -> CommandResult {
    if !state.frames.last().is_some_and(|frame| frame.function) {
        return CommandResult::Error("return: can only return from a function".to_string());
    }
    let status = match arguments {
        [] => state.last_status,
        [text] => match text.parse::<i64>() {
            Ok(code) => (code & 0xff) as i32,
            Err(_) => {
                return CommandResult::Error(format!("return: {text}: numeric argument required"));
            }
        },
        _ => return CommandResult::Error("return: too many arguments".to_string()),
    };
    state.returning = Some(status);
    CommandResult::Continue
}

/// `shift [count]`: drop the first positional parameters.
pub(super) fn run_shift(arguments: &[String], state: &mut ShellState) -> CommandResult {
    let Some(frame) = state.frames.last_mut() else {
        return CommandResult::Error("shift: there are no positional parameters".to_string());
    };
    let count = match arguments {
        [] => 1,
        [text] => match text.parse::<usize>() {
            Ok(count) => count,
            Err(_) => {
                return CommandResult::Error(format!("shift: {text}: numeric argument required"));
            }
        },
        _ => return CommandResult::Error("shift: too many arguments".to_string()),
    };
    if count > frame.positional.len() {
        return CommandResult::Error("shift: shift count out of range".to_string());
    }
    frame.positional.drain(..count);
    CommandResult::Continue
}

/// `local NAME[=value]...`: variables that end with the function.
pub(super) fn run_local(arguments: &[String], state: &mut ShellState) -> CommandResult {
    if !state.frames.last().is_some_and(|frame| frame.function) {
        return CommandResult::Error("local: can only be used in a function".to_string());
    }
    // Check every name first, so that a mistake changes nothing.
    for argument in arguments {
        let name = argument
            .split_once('=')
            .map_or(argument.as_str(), |(name, _)| name);
        if !shell_env::is_valid_name(name) {
            return CommandResult::Error(format!("local: not a valid name: {name}"));
        }
    }
    for argument in arguments {
        let (name, value) = match argument.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (argument.as_str(), None),
        };
        let previous = state.variables.get(name).cloned();
        // Declared twice, the value of each time is kept: they are put back last
        // one first, so the first value is the one that ends up in place.
        if let Some(frame) = state.frames.last_mut() {
            frame.locals.push((name.to_string(), previous));
        }
        match value {
            Some(value) => {
                state.variables.insert(name.to_string(), value.to_string());
            }
            None => {
                state.variables.remove(name);
            }
        }
    }
    CommandResult::Continue
}
