//! Command substitution, `$(line)` and `` `line` `` (see ADR-0014).
//!
//! The text of a segment is rewritten before it is parsed: each `$(line)` is
//! replaced by what `line` printed, quoted so that it can only ever become words
//! and never an operator. The normal parser then reads the result.

use std::cell::Cell;

use super::{CommandResult, ShellState, arithmetic, run_line};
use crate::app::parser::scan_substitution;
use crate::render;
use crate::utils::shell_env;

const MAX_DEPTH: u8 = 24;

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
                    let (expression, end) = arithmetic_body(&characters, position + 3)?;
                    // Substitutions inside the expression run first, as in other
                    // shells: `$(( $1 * $(fact 3) ))`. Quoting the text makes each
                    // result come back as plain text.
                    let expression = if expression.contains("$(") {
                        let quoted = expand(&format!("\"{expression}\""), state)?;
                        quoted[1..quoted.len() - 1].to_string()
                    } else {
                        expression
                    };
                    let value = arithmetic::evaluate(&expression, &|name| state.variable(name))?;
                    // A number has no blank or special character: it needs no quoting.
                    output.push_str(&value.to_string());
                    position = end;
                    position += 1;
                    continue;
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
            // Backquotes, the old spelling of `$(…)`: the text up to the next backquote
            // that is not escaped, with `\`` `\\` and `\$` unescaped.
            (Some('"') | None, '`') => {
                let mut inner = String::new();
                let mut index = position + 1;
                loop {
                    match characters.get(index) {
                        None => return Err("unterminated backquote command substitution".into()),
                        Some('`') => break,
                        Some('\\')
                            if matches!(characters.get(index + 1), Some('`' | '\\' | '$')) =>
                        {
                            inner.push(characters[index + 1]);
                            index += 2;
                        }
                        Some(&other) => {
                            inner.push(other);
                            index += 1;
                        }
                    }
                }
                position = index;
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

/// The text of a `$((...))` that starts at `from` (just after the two opening
/// parentheses), and the index of its last `)`.
fn arithmetic_body(characters: &[char], from: usize) -> Result<(String, usize), String> {
    let mut depth = 0_usize;
    let mut index = from;
    while let Some(&character) = characters.get(index) {
        match character {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            ')' if characters.get(index + 1) == Some(&')') => {
                return Ok((characters[from..index].iter().collect(), index + 1));
            }
            ')' => return Err("arithmetic: missing '))'".to_string()),
            _ => {}
        }
        index += 1;
    }
    Err("arithmetic: missing '))'".to_string())
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
    // A `return` inside a substitution ends the substitution, not the function.
    state.returning = None;
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
    fn the_body_of_an_arithmetic_expansion_ends_at_its_double_parenthesis() {
        let characters: Vec<char> = "(1+2)*3)) rest".chars().collect();
        assert_eq!(
            arithmetic_body(&characters, 0),
            Ok(("(1+2)*3".to_string(), 8))
        );
        let characters: Vec<char> = "1+2".chars().collect();
        assert!(arithmetic_body(&characters, 0).is_err());
        // `$((1+2) )` closes the group with a space between: not arithmetic.
        let characters: Vec<char> = "1+2) )".chars().collect();
        assert!(arithmetic_body(&characters, 0).is_err());
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
