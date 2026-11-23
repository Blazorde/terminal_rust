//! The `(( expression ))` command and the `let` built-in (see ADR-0033).
//!
//! Both evaluate integer expressions where `=`, `+=`, `++` and the comma are allowed, and
//! both succeed when the last value is not zero.

use super::{CommandResult, ShellState, arithmetic, substitution};
use crate::app::parser::{Stage, arithmetic_len};

/// What is between `((` and `))` when `text` is exactly one `(( … ))` command.
pub(super) fn body(text: &str) -> Option<&str> {
    let text = text.trim();
    let length = arithmetic_len(text.chars())?;
    (length == text.chars().count()).then(|| &text[2..text.len() - 2])
}

/// Run `(( body ))`: the status is 0 when the value is not zero, 1 otherwise.
pub(super) fn run_expression(body: &str, state: &mut ShellState) -> CommandResult {
    match substitution::expand(body, state).and_then(|body| arithmetic::statements(&body, state)) {
        Ok(0) => CommandResult::Error(String::new()),
        Ok(_) => CommandResult::Continue,
        Err(message) => CommandResult::Error(format!("((: {message}")),
    }
}

/// `let a=1 b=a+1`: each argument is an assignment or an expression; the status is 0 when
/// the last value is not zero.
pub(super) fn run_let(args: &[String], state: &mut ShellState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("let: expression expected".to_string());
    }
    let mut value = 0;
    for argument in args {
        match arithmetic::statements(argument, state) {
            Ok(result) => value = result,
            Err(message) => return CommandResult::Error(format!("let: {message}")),
        }
    }
    if value == 0 {
        CommandResult::Error(String::new())
    } else {
        CommandResult::Continue
    }
}

/// The stages of a pipeline where each `(( … ))` is rewritten into `let '…'`, so that the
/// pipeline code runs it like any built-in (pipes, redirections, background). `None` when no
/// stage is an arithmetic command.
pub(super) fn as_let_stages(stages: &[Stage]) -> Option<Vec<Stage>> {
    if !stages.iter().any(|stage| body(&stage.text).is_some()) {
        return None;
    }
    Some(
        stages
            .iter()
            .map(|stage| Stage {
                text: match body(&stage.text) {
                    Some(body) => format!("let '{}'", body.replace('\'', "'\\''")),
                    None => stage.text.clone(),
                },
                redirects: stage.redirects.clone(),
            })
            .collect(),
    )
}
