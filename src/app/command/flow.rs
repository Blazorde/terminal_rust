//! Control flow: `for`, `while`, `until`, `if`, `case`, groups and function
//! definitions (see ADR-0016 and ADR-0021).
//!
//! The segments of a line (`split_chain` makes one per command, and one per
//! keyword that starts a part of a compound command) are turned into a tree
//! first, so a mistake rejects the whole input before anything runs. Then the
//! tree is executed.

use super::{CommandResult, ShellState, assign_variable, run_segment};
use crate::app::parser::{Connector, Segment, Stage, split_chain, split_stages};
use crate::commands::external;
use crate::render;
use crate::utils::shell_env;

const KEYWORDS: [&str; 15] = [
    "for", "while", "until", "if", "then", "elif", "else", "fi", "do", "done", "case", "esac",
    ";;", "{", "}",
];

/// What `break` and `continue` leave for the loop that runs the command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LoopSignal {
    Break,
    Continue,
}

pub(super) enum Node {
    /// One command or pipeline, already checked.
    Simple {
        segment: Segment,
        stages: Vec<Stage>,
    },
    For {
        connector: Connector,
        variable: String,
        /// The raw words after `in`, expanded when the loop starts.
        words: String,
        body: Vec<Node>,
    },
    While {
        connector: Connector,
        until: bool,
        condition: Vec<Node>,
        body: Vec<Node>,
    },
    If {
        connector: Connector,
        branches: Vec<(Vec<Node>, Vec<Node>)>,
        otherwise: Vec<Node>,
    },
    Case {
        connector: Connector,
        /// The raw word that is matched, expanded when the `case` runs.
        word: String,
        /// For each arm, its raw patterns and its commands.
        arms: Vec<(Vec<String>, Vec<Node>)>,
    },
    /// `{ LIST; }`: commands run together.
    Group {
        connector: Connector,
        body: Vec<Node>,
    },
    /// `name() { LIST; }`: running it records the function.
    Function {
        connector: Connector,
        name: String,
        body: super::functions::Body,
    },
}

impl Node {
    fn connector(&self) -> Connector {
        match self {
            Self::Simple { segment, .. } => segment.connector,
            Self::For { connector, .. }
            | Self::While { connector, .. }
            | Self::If { connector, .. }
            | Self::Case { connector, .. }
            | Self::Group { connector, .. }
            | Self::Function { connector, .. } => *connector,
        }
    }
}

/// Whether `name` is a keyword of the shell language.
pub(super) fn is_keyword(name: &str) -> bool {
    KEYWORDS.contains(&name)
}

/// The keyword a command starts with and what follows it in the same command.
fn keyword(text: &str) -> Option<(&'static str, &str)> {
    let trimmed = text.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let word = &trimmed[..end];
    KEYWORDS
        .iter()
        .find(|candidate| **candidate == word)
        .map(|candidate| (*candidate, trimmed[end..].trim_start()))
}

/// Whether `line` opens a `for`, `while`, `until`, `if` or `case` that is not
/// closed yet: the input must go on over the next line.
pub(super) fn is_incomplete(line: &str) -> bool {
    let Ok(segments) = split_chain(line) else {
        return false;
    };
    let mut depth = 0_i32;
    // How many `case` are open: only inside one does a segment start with `pattern)`.
    let mut cases = 0_i32;
    // After `name()` on a line of its own, the `{` is still to come.
    let mut awaiting_brace = false;
    for segment in segments {
        let mut text = segment.text.as_str();
        if awaiting_brace {
            awaiting_brace = false;
        }
        // A segment can hold several openers in a row: `case x in a) for ...`.
        loop {
            if cases > 0
                && keyword(text).is_none()
                && let Some((_, rest)) = split_arm(text)
            {
                text = rest;
            }
            // `do for ...` and `then if ...` open a block in the same command.
            while let Some(("do" | "then" | "else", rest)) = keyword(text) {
                text = rest;
            }
            // `name() {` opens a block like `{` does.
            if keyword(text).is_none()
                && let Ok(Some(header)) = function_header(text)
            {
                if header.opens {
                    depth += 1;
                    if !header.rest.is_empty() {
                        text = header.rest;
                        continue;
                    }
                } else {
                    awaiting_brace = true;
                }
                break;
            }
            match keyword(text) {
                Some(("for" | "while" | "until" | "if", _)) => depth += 1,
                Some(("{", rest)) => {
                    depth += 1;
                    if !rest.is_empty() {
                        text = rest;
                        continue;
                    }
                }
                Some(("case", rest)) => {
                    depth += 1;
                    cases += 1;
                    // What follows `case WORD in` may be the first arm.
                    let (_, after_word) = split_word(rest);
                    if let ("in", arm) = split_first_word(after_word)
                        && !arm.is_empty()
                    {
                        text = arm;
                        continue;
                    }
                }
                Some(("done" | "fi" | "}", _)) => depth -= 1,
                Some(("esac", _)) => {
                    depth -= 1;
                    cases -= 1;
                }
                _ => {}
            }
            break;
        }
    }
    depth > 0 || awaiting_brace
}

/// A function definition at the start of a command: `name() {`, `name ()`,
/// `function name {` or `function name()`.
struct FunctionHeader<'a> {
    name: String,
    /// Whether the `{` that opens the body is already there; if not, it is the
    /// next command.
    opens: bool,
    /// What follows the `{` in the same command.
    rest: &'a str,
}

/// Whether a function name is allowed: letters, digits, `_` and `-`, not starting
/// with a digit or a dash, and no keyword.
fn is_function_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        && !KEYWORDS.contains(&name)
}

/// Read a function header from the start of `text`. `Ok(None)` when it is not
/// one, an error when it starts like one and is wrong.
fn function_header(text: &str) -> Result<Option<FunctionHeader<'_>>, String> {
    let text = text.trim_start();
    let long_form = text
        .strip_prefix("function")
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .map(str::trim_start);
    let (name, after) = match long_form {
        Some(rest) => {
            let end = rest
                .find(|character: char| character.is_whitespace() || character == '(')
                .unwrap_or(rest.len());
            let (name, mut after) = (&rest[..end], rest[end..].trim_start());
            if let Some(inner) = after.strip_prefix('(') {
                let Some(closed) = inner.trim_start().strip_prefix(')') else {
                    return Err("syntax error: expected ')' after 'function NAME ('".to_string());
                };
                after = closed.trim_start();
            }
            if !is_function_name(name) {
                return Err(format!("function: '{name}' is not a valid function name"));
            }
            (name, after)
        }
        None => {
            let end = text
                .find(|character: char| {
                    !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
                })
                .unwrap_or(text.len());
            let (name, after) = (&text[..end], text[end..].trim_start());
            let Some(inner) = after.strip_prefix('(') else {
                return Ok(None);
            };
            let Some(closed) = inner.trim_start().strip_prefix(')') else {
                return Ok(None);
            };
            if !is_function_name(name) {
                return if name.is_empty() {
                    Ok(None)
                } else {
                    Err(format!(
                        "syntax error: '{name}' is not a valid function name"
                    ))
                };
            }
            (name, closed.trim_start())
        }
    };
    if after.is_empty() {
        return Ok(Some(FunctionHeader {
            name: name.to_string(),
            opens: false,
            rest: "",
        }));
    }
    match after.strip_prefix('{') {
        Some(rest) if rest.is_empty() || rest.starts_with(char::is_whitespace) => {
            Ok(Some(FunctionHeader {
                name: name.to_string(),
                opens: true,
                rest: rest.trim_start(),
            }))
        }
        _ => Err(format!("syntax error: expected '{{' after '{name}()'")),
    }
}

/// The first word of `text`, quotes and `$(...)` included, and what follows it.
fn split_word(text: &str) -> (&str, &str) {
    let text = text.trim_start();
    let mut quote: Option<char> = None;
    let mut substitution = 0_usize;
    let mut previous = '\0';
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if escaped {
            escaped = false;
            previous = character;
            continue;
        }
        match (quote, character) {
            (_, '\\') if quote != Some('\'') => escaped = true,
            (Some(open), close) if open == close => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '(') if previous == '$' || substitution > 0 => substitution += 1,
            (None, ')') if substitution > 0 => substitution -= 1,
            (None, blank) if blank.is_whitespace() && substitution == 0 => {
                return (&text[..index], text[index..].trim_start());
            }
            _ => {}
        }
        previous = character;
    }
    (text, "")
}

/// An arm of a `case`: the patterns before the first `)` that is not quoted, and
/// what follows it. A `(` before the patterns is allowed and dropped.
fn split_arm(text: &str) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, character) {
            (_, '\\') if quote != Some('\'') => escaped = true,
            (Some(open), close) if open == close => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, ')') => {
                let patterns = text[..index].trim();
                let patterns = patterns.strip_prefix('(').unwrap_or(patterns).trim();
                return Some((patterns, text[index + 1..].trim_start()));
            }
            _ => {}
        }
    }
    None
}

/// The patterns of an arm, split at the `|` that are not quoted.
fn split_patterns(patterns: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for character in patterns.chars() {
        if escaped {
            escaped = false;
            current.push(character);
            continue;
        }
        match (quote, character) {
            (_, '\\') if quote != Some('\'') => {
                escaped = true;
                current.push(character);
            }
            (Some(open), close) if open == close => {
                quote = None;
                current.push(character);
            }
            (Some(_), _) => current.push(character),
            (None, '\'' | '"') => {
                quote = Some(character);
                current.push(character);
            }
            (None, '|') => parts.push(std::mem::take(&mut current).trim().to_string()),
            (None, _) => current.push(character),
        }
    }
    parts.push(current.trim().to_string());
    parts
}

struct Parser {
    segments: Vec<Segment>,
    position: usize,
}

/// Build the tree of a line, or say what is wrong with it.
pub(super) fn parse(segments: Vec<Segment>) -> Result<Vec<Node>, String> {
    let mut parser = Parser {
        segments,
        position: 0,
    };
    let nodes = parser.list(&[])?;
    match parser.current_keyword() {
        Some((word, _)) => Err(unexpected(word)),
        None => Ok(nodes),
    }
}

fn unexpected(word: &str) -> String {
    format!("syntax error near unexpected token '{word}'")
}

fn split_first_word(text: &str) -> (&str, &str) {
    let text = text.trim_start();
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    (&text[..end], text[end..].trim_start())
}

impl Parser {
    fn current_keyword(&self) -> Option<(&'static str, String)> {
        let segment = self.segments.get(self.position)?;
        keyword(&segment.text).map(|(word, rest)| (word, rest.to_string()))
    }

    /// Step over the keyword that starts the current command. What follows it
    /// stays as the command itself (`do echo hi` leaves `echo hi`).
    fn consume_keyword(&mut self) {
        let Some((_, rest)) = self.current_keyword() else {
            return;
        };
        if rest.is_empty() {
            self.position += 1;
        } else {
            self.segments[self.position].text = rest;
        }
    }

    fn expect(&mut self, wanted: &'static str) -> Result<(), String> {
        match self.current_keyword() {
            Some((word, _)) if word == wanted => {
                self.consume_keyword();
                Ok(())
            }
            Some((word, _)) => Err(format!(
                "syntax error: expected '{wanted}' but found '{word}'"
            )),
            None => Err(format!("syntax error: expected '{wanted}'")),
        }
    }

    /// The commands up to one of the `stops` keywords, which is left in place.
    fn list(&mut self, stops: &[&str]) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        while self.position < self.segments.len() {
            match self.current_keyword() {
                Some((word, _)) if stops.contains(&word) => return Ok(nodes),
                Some(("for", _)) => nodes.push(self.for_loop()?),
                Some(("while", _)) => nodes.push(self.while_loop(false)?),
                Some(("until", _)) => nodes.push(self.while_loop(true)?),
                Some(("if", _)) => nodes.push(self.conditional()?),
                Some(("case", _)) => nodes.push(self.case_statement()?),
                Some(("{", _)) => nodes.push(self.group()?),
                Some((word, _)) => return Err(unexpected(word)),
                None => {
                    if let Some(header) = function_header(&self.segments[self.position].text)? {
                        let (name, opens, rest) =
                            (header.name, header.opens, header.rest.to_string());
                        nodes.push(self.function_definition(name, opens, rest)?);
                        continue;
                    }
                    let segment = Segment {
                        connector: self.segments[self.position].connector,
                        text: self.segments[self.position].text.clone(),
                        background: self.segments[self.position].background,
                    };
                    let stages = split_stages(&segment.text)?;
                    nodes.push(Node::Simple { segment, stages });
                    self.position += 1;
                }
            }
        }
        Ok(nodes)
    }

    fn for_loop(&mut self) -> Result<Node, String> {
        let connector = self.segments[self.position].connector;
        let (_, header) = self
            .current_keyword()
            .ok_or_else(|| "syntax error: expected 'for'".to_string())?;
        let (name, after) = split_first_word(&header);
        if !shell_env::is_valid_name(name) {
            return Err(format!("for: '{name}' is not a valid variable name"));
        }
        let (word, words) = split_first_word(after);
        if word != "in" {
            return Err("syntax error: expected 'in' after 'for NAME'".to_string());
        }
        let (variable, words) = (name.to_string(), words.to_string());
        self.position += 1;
        self.expect("do")?;
        let body = self.list(&["done"])?;
        self.finish("done")?;
        Ok(Node::For {
            connector,
            variable,
            words,
            body,
        })
    }

    fn while_loop(&mut self, until: bool) -> Result<Node, String> {
        let connector = self.segments[self.position].connector;
        self.consume_keyword();
        let condition = self.list(&["do"])?;
        self.expect("do")?;
        let body = self.list(&["done"])?;
        self.finish("done")?;
        Ok(Node::While {
            connector,
            until,
            condition,
            body,
        })
    }

    fn conditional(&mut self) -> Result<Node, String> {
        let connector = self.segments[self.position].connector;
        self.consume_keyword();
        let mut branches = Vec::new();
        let mut otherwise = Vec::new();
        loop {
            let condition = self.list(&["then"])?;
            self.expect("then")?;
            let body = self.list(&["elif", "else", "fi"])?;
            branches.push((condition, body));
            match self.current_keyword() {
                Some(("elif", _)) => self.consume_keyword(),
                Some(("else", _)) => {
                    self.consume_keyword();
                    otherwise = self.list(&["fi"])?;
                    break;
                }
                _ => break,
            }
        }
        self.finish("fi")?;
        Ok(Node::If {
            connector,
            branches,
            otherwise,
        })
    }

    /// `{ LIST; }`.
    fn group(&mut self) -> Result<Node, String> {
        let connector = self.segments[self.position].connector;
        self.consume_keyword();
        let body = self.list(&["}"])?;
        self.finish("}")?;
        Ok(Node::Group { connector, body })
    }

    /// `name() { LIST; }`: the header was read, and `opens` says whether the `{` is
    /// already consumed (`rest` is what followed it) or comes next.
    fn function_definition(
        &mut self,
        name: String,
        opens: bool,
        rest: String,
    ) -> Result<Node, String> {
        let connector = self.segments[self.position].connector;
        if opens {
            if rest.is_empty() {
                self.position += 1;
            } else {
                self.segments[self.position].text = rest;
            }
        } else {
            self.position += 1;
            self.expect("{")?;
        }
        let body = self.list(&["}"])?;
        self.finish("}")?;
        Ok(Node::Function {
            connector,
            name,
            body: std::sync::Arc::new(body),
        })
    }

    /// `case WORD in PATTERN) LIST ;; ... esac`.
    fn case_statement(&mut self) -> Result<Node, String> {
        let connector = self.segments[self.position].connector;
        let (_, header) = self
            .current_keyword()
            .ok_or_else(|| "syntax error: expected 'case'".to_string())?;
        let (word, after) = split_word(&header);
        if word.is_empty() {
            return Err("syntax error: expected a word after 'case'".to_string());
        }
        let (in_word, rest) = split_first_word(after);
        if in_word != "in" {
            return Err("syntax error: expected 'in' after 'case WORD'".to_string());
        }
        let (word, rest) = (word.to_string(), rest.to_string());
        if rest.is_empty() {
            self.position += 1;
        } else {
            self.segments[self.position].text = rest;
        }

        let mut arms = Vec::new();
        loop {
            match self.current_keyword() {
                Some(("esac", _)) => {
                    self.finish("esac")?;
                    break;
                }
                // An empty arm, or a stray `;;` right after another.
                Some((";;", _)) => self.position += 1,
                Some((other, _)) => return Err(unexpected(other)),
                None if self.position >= self.segments.len() => {
                    return Err("syntax error: expected 'esac'".to_string());
                }
                None => {
                    let text = self.segments[self.position].text.clone();
                    let Some((patterns, rest)) = split_arm(&text) else {
                        return Err("syntax error: expected 'PATTERN)' in 'case'".to_string());
                    };
                    let patterns = split_patterns(patterns);
                    if patterns.iter().any(String::is_empty) {
                        return Err("syntax error: empty pattern in 'case'".to_string());
                    }
                    if rest.is_empty() {
                        self.position += 1;
                    } else {
                        self.segments[self.position].text = rest.to_string();
                    }
                    let body = self.list(&[";;", "esac"])?;
                    arms.push((patterns, body));
                    if matches!(self.current_keyword(), Some((";;", _))) {
                        self.position += 1;
                    }
                }
            }
        }
        Ok(Node::Case {
            connector,
            word,
            arms,
        })
    }

    /// The keyword that closes a compound command, with nothing after it.
    fn finish(&mut self, wanted: &'static str) -> Result<(), String> {
        match self.current_keyword() {
            Some((word, rest)) if word == wanted => {
                if !rest.is_empty() {
                    return Err(format!("syntax error: unexpected text after '{wanted}'"));
                }
                self.position += 1;
                Ok(())
            }
            _ => Err(format!("syntax error: expected '{wanted}'")),
        }
    }
}

/// The result of a command that `Ctrl+C` stopped: status 130, nothing to print.
fn interrupted_result() -> CommandResult {
    external::note_exit_code(130);
    CommandResult::Error(String::new())
}

/// Run the nodes of a list in order, applying `&&` and `||` between them.
pub(super) fn run_nodes(nodes: &[Node], state: &mut ShellState) -> CommandResult {
    let mut last = CommandResult::Continue;
    for node in nodes {
        if state.loop_signal.is_some() || state.returning.is_some() {
            break;
        }
        if external::interrupt_requested() {
            return interrupted_result();
        }
        let failed = matches!(last, CommandResult::Error(_));
        let run = match node.connector() {
            Connector::Then => true,
            Connector::And => !failed,
            Connector::Or => failed,
        };
        if !run {
            continue;
        }
        report(&last);

        last = match node {
            Node::Simple { segment, stages } => {
                let result = run_segment(&segment.text, stages, segment.background, state);
                state.record_status(matches!(result, CommandResult::Error(_)));
                result
            }
            Node::For {
                variable,
                words,
                body,
                ..
            } => run_for(variable, words, body, state),
            Node::While {
                until,
                condition,
                body,
                ..
            } => run_while(*until, condition, body, state),
            Node::If {
                branches,
                otherwise,
                ..
            } => run_if(branches, otherwise, state),
            Node::Case { word, arms, .. } => run_case(word, arms, state),
            Node::Group { body, .. } => run_nodes(body, state),
            Node::Function { name, body, .. } => {
                state
                    .functions
                    .insert(name.clone(), std::sync::Arc::clone(body));
                state.record_status(false);
                CommandResult::Continue
            }
        };
        if last == CommandResult::Exit {
            break;
        }
    }
    last
}

/// Print the error of a command that another one is about to follow.
fn report(result: &CommandResult) {
    if let CommandResult::Error(message) = result
        && !message.is_empty()
    {
        let _ = render::stderr_line(message);
    }
}

fn run_for(variable: &str, words: &str, body: &[Node], state: &mut ShellState) -> CommandResult {
    let items = match super::expand_words(words, state) {
        Ok(items) => items,
        Err(error) => return CommandResult::Error(error),
    };
    let mut last = CommandResult::Continue;
    state.loop_depth += 1;
    for (index, item) in items.iter().enumerate() {
        if external::interrupt_requested() {
            last = interrupted_result();
            break;
        }
        if index > 0 {
            report(&last);
        }
        assign_variable(variable, item, state);
        last = run_nodes(body, state);
        if finished(&last, state) {
            break;
        }
    }
    state.loop_depth -= 1;
    state.loop_signal = None;
    last
}

fn run_while(
    until: bool,
    condition: &[Node],
    body: &[Node],
    state: &mut ShellState,
) -> CommandResult {
    let mut last = CommandResult::Continue;
    state.loop_depth += 1;
    let mut turns = 0_usize;
    // The status of the loop is the one of the last command of the body, not the
    // one of the condition that ended it.
    let mut status = 0;
    loop {
        if external::interrupt_requested() {
            last = interrupted_result();
            break;
        }
        let (holds, exit) = test(condition, state);
        if exit {
            last = CommandResult::Exit;
            break;
        }
        if holds == until {
            break;
        }
        if turns > 0 {
            report(&last);
        }
        turns += 1;
        last = run_nodes(body, state);
        status = state.last_status;
        if finished(&last, state) {
            break;
        }
    }
    state.loop_depth -= 1;
    state.loop_signal = None;
    state.last_status = status;
    last
}

/// Whether the loop is over after a turn: `break`, or `exit`. `continue` is
/// consumed here, so the next turn starts clean.
fn finished(last: &CommandResult, state: &mut ShellState) -> bool {
    // `return` ends every loop of the function; the flag is left for the call.
    if *last == CommandResult::Exit || state.returning.is_some() {
        return true;
    }
    matches!(state.loop_signal.take(), Some(LoopSignal::Break))
}

/// Run a condition: whether it succeeded, and whether it asked to leave the
/// shell. A failing test is not an error, so its message is not shown, except
/// for a command that does not exist.
fn test(condition: &[Node], state: &mut ShellState) -> (bool, bool) {
    let result = run_nodes(condition, state);
    match &result {
        CommandResult::Exit => (false, true),
        CommandResult::Error(message) => {
            if message.starts_with("unknown command") {
                let _ = render::stderr_line(message);
            }
            (false, false)
        }
        CommandResult::Continue => (true, false),
    }
}

fn run_if(
    branches: &[(Vec<Node>, Vec<Node>)],
    otherwise: &[Node],
    state: &mut ShellState,
) -> CommandResult {
    for (condition, body) in branches {
        let (holds, exit) = test(condition, state);
        if exit {
            return CommandResult::Exit;
        }
        if holds {
            return run_nodes(body, state);
        }
    }
    if otherwise.is_empty() {
        // No branch ran: the status is 0, not the one of the last test.
        state.record_status(false);
        return CommandResult::Continue;
    }
    run_nodes(otherwise, state)
}

fn run_case(
    word: &str,
    arms: &[(Vec<String>, Vec<Node>)],
    state: &mut ShellState,
) -> CommandResult {
    let subject = match super::expand_case_word(word, state) {
        Ok(subject) => subject,
        Err(error) => return CommandResult::Error(error),
    };
    for (patterns, body) in arms {
        for pattern in patterns {
            let pattern = match super::expand_case_pattern(pattern, state) {
                Ok(pattern) => pattern,
                Err(error) => return CommandResult::Error(error),
            };
            if crate::utils::glob::matches_text(&pattern, &subject) {
                return run_nodes(body, state);
            }
        }
    }
    // No arm matched: the status is 0.
    state.record_status(false);
    CommandResult::Continue
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(line: &str) -> Result<Vec<Node>, String> {
        parse(split_chain(line)?)
    }

    fn shape(nodes: &[Node]) -> String {
        nodes
            .iter()
            .map(|node| match node {
                Node::Simple { segment, .. } => segment.text.trim().to_string(),
                Node::For {
                    variable,
                    words,
                    body,
                    ..
                } => format!("for {variable} in [{words}] do ({})", shape(body)),
                Node::While {
                    until,
                    condition,
                    body,
                    ..
                } => format!(
                    "{} ({}) do ({})",
                    if *until { "until" } else { "while" },
                    shape(condition),
                    shape(body)
                ),
                Node::If {
                    branches,
                    otherwise,
                    ..
                } => {
                    let parts: Vec<String> = branches
                        .iter()
                        .map(|(condition, body)| {
                            format!("({}) then ({})", shape(condition), shape(body))
                        })
                        .collect();
                    format!("if {} else ({})", parts.join(" elif "), shape(otherwise))
                }
                Node::Group { body, .. } => format!("group ({})", shape(body)),
                Node::Function { name, body, .. } => format!("function {name} ({})", shape(body)),
                Node::Case { word, arms, .. } => {
                    let parts: Vec<String> = arms
                        .iter()
                        .map(|(patterns, body)| {
                            format!("{} => ({})", patterns.join("|"), shape(body))
                        })
                        .collect();
                    format!("case {word} [{}]", parts.join("; "))
                }
            })
            .collect::<Vec<_>>()
            .join(" ; ")
    }

    fn shaped(line: &str) -> String {
        shape(&tree(line).expect("the line should parse"))
    }

    #[test]
    fn a_plain_command_is_a_simple_node() {
        assert_eq!(shaped("echo a ; echo b"), "echo a ; echo b");
    }

    #[test]
    fn a_for_loop_on_one_line() {
        assert_eq!(
            shaped("for f in a b c; do echo $f; done"),
            "for f in [a b c] do (echo $f)"
        );
        assert_eq!(
            shaped("for f in a; do echo 1; echo 2; done ; echo after"),
            "for f in [a] do (echo 1 ; echo 2) ; echo after"
        );
        assert_eq!(
            shaped("for f in; do echo x; done"),
            "for f in [] do (echo x)"
        );
    }

    #[test]
    fn the_same_loop_over_several_lines() {
        assert_eq!(
            shaped("for f in a b\ndo\n  echo $f\ndone"),
            "for f in [a b] do (echo $f)"
        );
    }

    #[test]
    fn while_and_until_loops() {
        assert_eq!(
            shaped("while test -f x; do rm x; done"),
            "while (test -f x) do (rm x)"
        );
        assert_eq!(shaped("until a && b; do c; done"), "until (a ; b) do (c)");
    }

    #[test]
    fn conditionals_with_elif_and_else() {
        assert_eq!(shaped("if a; then b; fi"), "if (a) then (b) else ()");
        assert_eq!(
            shaped("if a; then b; elif c; then d; else e; fi"),
            "if (a) then (b) elif (c) then (d) else (e)"
        );
        assert_eq!(
            shaped("if a\nthen\n b\nelse\n c\nfi"),
            "if (a) then (b) else (c)"
        );
    }

    #[test]
    fn compound_commands_nest_and_follow_each_other() {
        assert_eq!(
            shaped("for a in 1 2; do for b in x y; do echo $a$b; done; done"),
            "for a in [1 2] do (for b in [x y] do (echo $a$b))"
        );
        assert_eq!(
            shaped("for a in 1; do if t; then echo y; fi; done; echo end"),
            "for a in [1] do (if (t) then (echo y) else ()) ; echo end"
        );
    }

    #[test]
    fn a_keyword_inside_a_word_or_after_an_argument_is_ordinary() {
        assert_eq!(shaped("echo for while done"), "echo for while done");
        assert_eq!(shaped("echo done; echo fi"), "echo done ; echo fi");
        assert_eq!(shaped("\"for\" a"), "\"for\" a");
    }

    #[test]
    fn syntax_errors_name_what_is_missing() {
        for (line, expected) in [
            ("for f in a; do echo", "syntax error: expected 'done'"),
            ("for f in a; echo x; done", "syntax error: expected 'do'"),
            ("for f a; do x; done", "expected 'in' after 'for NAME'"),
            ("for 1x in a; do x; done", "not a valid variable name"),
            ("if a; then b", "syntax error: expected 'fi'"),
            ("if a; b; fi", "unexpected token 'fi'"),
            ("done", "unexpected token 'done'"),
            ("then echo", "unexpected token 'then'"),
            ("while a; do b; done done", "unexpected text after 'done'"),
            ("while a; do b; done; done", "unexpected token 'done'"),
            (
                "for f in a; do x; done > out",
                "unexpected text after 'done'",
            ),
        ] {
            let error = tree(line)
                .err()
                .unwrap_or_else(|| panic!("{line} should fail"));
            assert!(error.contains(expected), "{line}: {error}");
        }
    }

    #[test]
    fn a_case_is_parsed_with_its_arms_on_one_line_or_several() {
        assert_eq!(
            shape(&tree("case $x in a) echo A;; b|c) echo B;; *) echo other;; esac").unwrap()),
            "case $x [a => (echo A); b|c => (echo B); * => (echo other)]"
        );
        assert_eq!(
            shape(&tree("case $x in\n a)\n echo A\n echo A2\n ;;\n *) echo other\nesac").unwrap()),
            "case $x [a => (echo A ; echo A2); * => (echo other)]"
        );
        // The last `;;` is optional, an arm may be empty, and `(` may open the patterns.
        assert_eq!(
            shape(&tree("case x in (a) echo A ;; b) ;; esac").unwrap()),
            "case x [a => (echo A); b => ()]"
        );
        assert_eq!(shape(&tree("case x in esac").unwrap()), "case x []");
    }

    #[test]
    fn a_case_nests_and_holds_other_blocks() {
        assert_eq!(
            shape(
                &tree("case a in a) for i in 1 2; do echo $i; done ;; b) if true; then echo y; fi ;; esac")
                    .unwrap()
            ),
            "case a [a => (for i in [1 2] do (echo $i)); b => (if (true) then (echo y) else ())]"
        );
        assert_eq!(
            shape(&tree("case a in a) case b in b) echo deep ;; esac ;; esac").unwrap()),
            "case a [a => (case b [b => (echo deep)])]"
        );
        // A quoted word with blanks, and a substitution, are one word.
        assert_eq!(
            shape(&tree("case \"a b\" in *) echo x;; esac").unwrap()),
            "case \"a b\" [* => (echo x)]"
        );
        assert_eq!(
            shape(&tree("case $(echo a b) in *) echo x;; esac").unwrap()),
            "case $(echo a b) [* => (echo x)]"
        );
        // A `)` inside quotes does not end the patterns.
        assert_eq!(
            shape(&tree("case x in \")\") echo p;; esac").unwrap()),
            "case x [\")\" => (echo p)]"
        );
    }

    #[test]
    fn a_malformed_case_is_rejected() {
        for line in [
            "case x esac",
            "case in a) echo;; esac",
            "case x in a) echo A",
            "case x in echo A;; esac",
            "case x in a|) echo;; esac",
            "case x in |a) echo;; esac",
            "echo a ;; echo b",
            "esac",
            "case x in a) echo;; fi esac",
        ] {
            assert!(tree(line).is_err(), "{line:?} should be rejected");
        }
    }

    #[test]
    fn an_unfinished_case_asks_for_more_input() {
        assert!(is_incomplete("case x in"));
        assert!(is_incomplete("case x in\n a) echo A ;;"));
        assert!(is_incomplete("case x in a) for i in 1; do echo $i"));
        assert!(is_incomplete("case x in a) if true; then echo y"));
        assert!(!is_incomplete("case x in a) echo A ;; esac"));
        assert!(!is_incomplete(
            "case x in a) for i in 1; do echo $i; done ;; esac"
        ));
        assert!(is_incomplete(
            "for i in 1; do case $i in 1) echo one ;; esac"
        ));
        assert!(!is_incomplete("echo done"));
        // A block opened in an arm and never closed keeps the input open even
        // when the `esac` is there.
        assert!(is_incomplete("case x in a) for i in 1; do echo $i ;; esac"));
        assert!(is_incomplete("case x in a) if true; then echo y ;; esac"));
    }

    #[test]
    fn a_function_is_parsed_in_all_its_forms() {
        let expected = "function greet (echo hi)";
        for line in [
            "greet() { echo hi; }",
            "greet () { echo hi; }",
            "greet(){ echo hi; }",
            "function greet { echo hi; }",
            "function greet() { echo hi; }",
            "function greet ( ) { echo hi; }",
            "greet()\n{\n  echo hi\n}",
            "greet() {\n  echo hi\n}",
            "function greet\n{\n echo hi\n}",
        ] {
            assert_eq!(shape(&tree(line).unwrap()), expected, "{line:?}");
        }
        assert_eq!(
            shape(&tree("my-func_2() { echo a; echo b; }").unwrap()),
            "function my-func_2 (echo a ; echo b)"
        );
        assert_eq!(shape(&tree("f() { }").unwrap()), "function f ()");
    }

    #[test]
    fn a_group_and_nested_blocks_are_parsed() {
        assert_eq!(
            shape(&tree("{ echo a; echo b; } && echo c").unwrap()),
            "group (echo a ; echo b) ; echo c"
        );
        assert_eq!(
            shape(&tree("f() { g() { echo inner; }; g; }").unwrap()),
            "function f (function g (echo inner) ; g)"
        );
        assert_eq!(
            shape(&tree("f() { for i in 1 2; do case $i in 1) echo one;; esac; done; }").unwrap()),
            "function f (for i in [1 2] do (case $i [1 => (echo one)]))"
        );
        // A word that only looks like a header stays a command.
        assert_eq!(shape(&tree("foo(bar)").unwrap()), "foo(bar)");
        // `echo ( )` is a header with no body, as in other shells: an error.
        assert!(tree("echo ( )").is_err());
    }

    #[test]
    fn a_malformed_function_is_rejected() {
        for line in [
            "f() echo hi",
            "f() { echo hi",
            "f()",
            "function 1bad { echo; }",
            "function for { echo; }",
            "function { echo; }",
            "1f() { echo; }",
            "for() { echo; }",
            "f() { echo; } extra",
            "function f ( { echo; }",
            "}",
            "{ echo",
            "{echo hi; }",
        ] {
            assert!(tree(line).is_err(), "{line:?} should be rejected");
        }
    }

    #[test]
    fn an_unfinished_function_or_group_asks_for_more_input() {
        assert!(is_incomplete("f() {"));
        assert!(is_incomplete("f() { echo hi"));
        assert!(is_incomplete("function f {"));
        assert!(is_incomplete("{ echo hi"));
        assert!(is_incomplete("f()"));
        assert!(is_incomplete("f()\n{"));
        assert!(is_incomplete("f() { for i in 1; do echo $i; done"));
        assert!(is_incomplete("f() { if true; then echo y; fi"));
        assert!(!is_incomplete("f() { echo hi; }"));
        assert!(!is_incomplete("f()\n{\n echo hi\n}"));
        assert!(!is_incomplete("{ echo hi; }"));
        assert!(!is_incomplete("f() { g() { echo x; }; }"));
        assert!(!is_incomplete("echo }"));
    }

    #[test]
    fn an_unfinished_block_asks_for_more_input() {
        for line in [
            "for f in a b; do",
            "for f in a b\ndo\n echo $f",
            "while true; do echo x;",
            "if a; then b",
            "if a; then for x in 1; do y; done",
            "for a in 1; do for b in 2; do echo; done",
        ] {
            assert!(is_incomplete(line), "{line}");
        }
        for line in [
            "echo hi",
            "for f in a; do echo; done",
            "if a; then b; fi",
            "echo for",
            "done",
            "",
        ] {
            assert!(!is_incomplete(line), "{line}");
        }
    }
}
