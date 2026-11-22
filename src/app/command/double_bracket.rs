//! The `[[ expression ]]` conditional (see ADR-0026).
//!
//! Unlike `[`, the words are neither split nor matched against file names, `&&`, `||`,
//! `!` and parentheses group the tests, `==` and `!=` compare with a pattern, `=~` with
//! a regular expression, and `<` and `>` compare strings. The tests that look at files
//! or integers are the ones of `test` (ADR-0016).

use super::{ShellState, expand_case_pattern, expand_case_word};
use crate::app::parser::scan_substitution;
use crate::commands::test_command;
use crate::utils::glob;

/// What is between `[[` and the closing `]]`, when `text` is a `[[ … ]]` command.
pub(super) fn body(text: &str) -> Option<Result<&str, String>> {
    let rest = text.trim().strip_prefix("[[")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim();
    Some(match rest.strip_suffix("]]") {
        Some(inner) if inner.is_empty() || inner.ends_with(char::is_whitespace) => Ok(inner.trim()),
        _ => Err("[[: missing ']]'".to_string()),
    })
}

/// Evaluate the body of a `[[ … ]]`.
pub(super) fn evaluate(body: &str, state: &mut ShellState) -> test_command::Outcome {
    let tokens = split_words(body)?;
    if tokens.is_empty() {
        return Ok(false);
    }
    let mut parser = Parser {
        tokens: &tokens,
        position: 0,
        state,
    };
    let value = parser.or(true)?;
    match parser.tokens.get(parser.position) {
        None => Ok(value),
        Some(extra) => Err(format!("[[: unexpected '{extra}'")),
    }
}

/// The raw words of the body: split at unquoted blanks, quotes and substitutions kept.
fn split_words(text: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        match (quote, character) {
            (Some(open), close) if open == close => {
                quote = None;
                current.push(character);
            }
            (Some('"'), '\\') | (None, '\\') => {
                current.push(character);
                current.extend(characters.next());
            }
            (Some('\''), _) => current.push(character),
            (Some(_) | None, '$') => {
                current.push(character);
                let mut lookahead = characters.clone();
                if lookahead.next() == Some('(') {
                    current.extend(characters.next());
                    scan_substitution(|| characters.next(), |c| current.push(c))?;
                }
            }
            (Some(_), _) => current.push(character),
            (None, '\'' | '"') => {
                quote = Some(character);
                current.push(character);
            }
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            (None, _) => current.push(character),
        }
    }
    if quote.is_some() {
        return Err("missing closing quote".to_string());
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(words)
}

const UNARY: [&str; 14] = [
    "-e", "-f", "-d", "-L", "-h", "-r", "-w", "-x", "-s", "-z", "-n", "-t", "-b", "-c",
];
const INTEGER: [&str; 6] = ["-eq", "-ne", "-lt", "-le", "-gt", "-ge"];
const STRING: [&str; 6] = ["==", "=", "!=", "<", ">", "=~"];

struct Parser<'a> {
    tokens: &'a [String],
    position: usize,
    state: &'a mut ShellState,
}

impl Parser<'_> {
    fn peek(&self, offset: usize) -> Option<&str> {
        self.tokens.get(self.position + offset).map(String::as_str)
    }

    /// `a || b`. With `run` false the words are parsed and nothing is evaluated, so a
    /// substitution on the side that is skipped does not run.
    fn or(&mut self, run: bool) -> test_command::Outcome {
        let mut value = self.and(run)?;
        while self.peek(0) == Some("||") {
            self.position += 1;
            let right = self.and(run && !value)?;
            value = value || right;
        }
        Ok(value)
    }

    fn and(&mut self, run: bool) -> test_command::Outcome {
        let mut value = self.not(run)?;
        while self.peek(0) == Some("&&") {
            self.position += 1;
            let right = self.not(run && value)?;
            value = value && right;
        }
        Ok(value)
    }

    fn not(&mut self, run: bool) -> test_command::Outcome {
        if self.peek(0) == Some("!") {
            self.position += 1;
            return Ok(!self.not(run)?);
        }
        self.primary(run)
    }

    fn primary(&mut self, run: bool) -> test_command::Outcome {
        let Some(first) = self.peek(0).map(str::to_string) else {
            return Err("[[: expression expected".to_string());
        };
        if first == "(" {
            self.position += 1;
            let value = self.or(run)?;
            if self.peek(0) != Some(")") {
                return Err("[[: missing ')'".to_string());
            }
            self.position += 1;
            return Ok(value);
        }
        if UNARY.contains(&first.as_str())
            && self
                .peek(1)
                .is_some_and(|next| !matches!(next, "&&" | "||" | ")"))
        {
            let operand = self.tokens[self.position + 1].clone();
            self.position += 2;
            if !run {
                return Ok(false);
            }
            let operand = self.word(&operand)?;
            return test_command::evaluate(&[first, operand], false);
        }
        let operator = self.peek(1).map(str::to_string);
        if let Some(operator) = operator
            && (STRING.contains(&operator.as_str()) || INTEGER.contains(&operator.as_str()))
        {
            let Some(right) = self.peek(2).map(str::to_string) else {
                return Err(format!("[[: expected a word after '{operator}'"));
            };
            self.position += 3;
            if !run {
                return Ok(false);
            }
            let left = self.word(&first)?;
            return self.binary(&left, &operator, &right);
        }
        self.position += 1;
        if !run {
            return Ok(false);
        }
        Ok(!self.word(&first)?.is_empty())
    }

    /// A word, expanded: no splitting and no file name matching.
    fn word(&mut self, raw: &str) -> Result<String, String> {
        expand_case_word(raw, self.state)
    }

    fn binary(&mut self, left: &str, operator: &str, right: &str) -> test_command::Outcome {
        match operator {
            "==" | "=" | "!=" => {
                // What was typed as a wildcard matches; what was quoted stands for itself.
                let pattern = expand_case_pattern(right, self.state)?;
                let matched = glob::matches_text(&pattern, left);
                Ok(matched == (operator != "!="))
            }
            "=~" => {
                let expression = self.word(right)?;
                let expression = regex::Regex::new(&expression)
                    .map_err(|error| format!("[[: invalid regular expression: {error}"))?;
                Ok(expression.is_match(left))
            }
            "<" => Ok(left < self.word(right)?.as_str()),
            ">" => Ok(left > self.word(right)?.as_str()),
            _ => {
                let right = self.word(right)?;
                test_command::evaluate(&[left.to_string(), operator.to_string(), right], false)
            }
        }
    }
}
