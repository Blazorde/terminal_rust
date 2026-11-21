//! Command substitution, `$(line)` (see ADR-0014).
//!
//! The text of a segment is rewritten before it is parsed: each `$(line)` is
//! replaced by what `line` printed, quoted so that it can only ever become words
//! and never an operator. The normal parser then reads the result.

use std::cell::Cell;

use super::{CommandResult, ShellState, run_line};
use crate::app::parser::scan_substitution;
use crate::render;
use crate::utils::shell_env;

const MAX_DEPTH: u8 = 8;

thread_local! {
    static DEPTH: Cell<u8> = const { Cell::new(0) };
}

/// Replace every active `$(...)` of `text` by its output.
pub(super) fn expand(text: &str, state: &mut ShellState) -> Result<String, String> {
    let characters: Vec<char> = text.chars().collect();
    let mut output = String::with_capacity(text.len());
    let mut word_start = 0;
    let mut quote: Option<char> = None;
    let mut position = 0;

    while position < characters.len() {
        let character = characters[position];
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => {
                quote = None;
                output.push(character);
            }
            (Some('\''), _) => output.push(character),
            (Some('"'), '\\') | (None, '\\') => {
                output.push(character);
                if let Some(&escaped) = characters.get(position + 1) {
                    output.push(escaped);
                    position += 1;
                }
            }
            (Some('"') | None, '$') if characters.get(position + 1) == Some(&'(') => {
                if characters.get(position + 2) == Some(&'(') {
                    return Err("arithmetic expansion $(( )) is not supported".to_string());
                }
                position += 2;
                let mut inner = String::new();
                let mut index = position;
                scan_substitution(
                    || {
                        let next = characters.get(index).copied();
                        index += 1;
                        next
                    },
                    |c| inner.push(c),
                )?;
                position = index - 1;
                inner.pop();
                let result = capture(&inner, state)?;
                let in_assignment = quote.is_none() && is_assignment(&output[word_start..]);
                output.push_str(&replacement(&result, quote == Some('"'), in_assignment));
            }
            (None, '\'') => {
                quote = Some('\'');
                output.push(character);
            }
            (None, '"') => {
                quote = Some('"');
                output.push(character);
            }
            (None, c) if c.is_whitespace() => {
                output.push(character);
                word_start = output.len();
            }
            (_, _) => output.push(character),
        }
        position += 1;
    }
    Ok(output)
}

/// Whether the word typed so far is `NAME=`: its substitutions are not split.
fn is_assignment(word: &str) -> bool {
    word.split_once('=')
        .is_some_and(|(name, _)| shell_env::is_valid_name(name))
}

/// The text that stands for `result` in the rewritten line.
fn replacement(result: &str, in_double_quotes: bool, in_assignment: bool) -> String {
    if in_double_quotes {
        return escape(result);
    }
    if in_assignment {
        return format!("\"{}\"", escape(result));
    }
    // Unquoted: one word per run of non-blank characters, each kept literal.
    result
        .split_whitespace()
        .map(|word| format!("\"{}\"", escape(word)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Escape what a double-quoted word treats specially.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '"' | '$') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// Run `line` and return what it printed, without its trailing newlines.
fn capture(line: &str, state: &mut ShellState) -> Result<String, String> {
    let depth = DEPTH.with(Cell::get);
    if depth >= MAX_DEPTH {
        return Err("command substitution is nested too deeply".to_string());
    }
    DEPTH.with(|cell| cell.set(depth + 1));
    let (result, captured) = render::capture_substitution(|| run_line(line, state));
    DEPTH.with(|cell| cell.set(depth));

    // Diagnostics are for the user, not for the command that uses the result.
    if !captured.stderr.is_empty() {
        let _ = render::stderr(&captured.stderr);
    }
    if let CommandResult::Error(message) = &result
        && !message.is_empty()
    {
        let _ = render::stderr_line(message);
    }
    Ok(captured.stdout.trim_end_matches(['\n', '\r']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_stands_for_itself_between_quotes() {
        assert_eq!(replacement("a b", false, false), "\"a\" \"b\"");
        assert_eq!(replacement("", false, false), "");
        assert_eq!(replacement("  \n ", false, false), "");
    }

    #[test]
    fn double_quotes_keep_the_result_as_it_is() {
        assert_eq!(replacement("a  b\nc", true, false), "a  b\nc");
    }

    #[test]
    fn an_assignment_keeps_one_word() {
        assert_eq!(replacement("a b", false, true), "\"a b\"");
    }

    #[test]
    fn what_a_quoted_word_treats_specially_is_escaped() {
        assert_eq!(escape("a\"b$c\\d"), "a\\\"b\\$c\\\\d");
        assert_eq!(replacement("x;y|z>w", false, false), "\"x;y|z>w\"");
    }

    #[test]
    fn an_assignment_word_is_recognised() {
        assert!(is_assignment("NAME="));
        assert!(is_assignment("export_x=a"));
        assert!(!is_assignment("echo"));
        assert!(!is_assignment("1x="));
        assert!(!is_assignment("--flag="));
    }
}
