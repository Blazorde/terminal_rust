//! Control flow: `for`, `while`, `until` and `if` (see ADR-0016).
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

const KEYWORDS: [&str; 10] = [
    "for", "while", "until", "if", "then", "elif", "else", "fi", "do", "done",
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
}

impl Node {
    fn connector(&self) -> Connector {
        match self {
            Self::Simple { segment, .. } => segment.connector,
            Self::For { connector, .. }
            | Self::While { connector, .. }
            | Self::If { connector, .. } => *connector,
        }
    }
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

/// Whether `line` opens a `for`, `while`, `until` or `if` that is not closed yet:
/// the input must go on over the next line.
pub(super) fn is_incomplete(line: &str) -> bool {
    let Ok(segments) = split_chain(line) else {
        return false;
    };
    let mut depth = 0_i32;
    for segment in segments {
        let mut text = segment.text.as_str();
        // `do for ...` and `then if ...` open a block in the same command.
        while let Some(("do" | "then" | "else", rest)) = keyword(text) {
            text = rest;
        }
        match keyword(text) {
            Some(("for" | "while" | "until" | "if", _)) => depth += 1,
            Some(("done" | "fi", _)) => depth -= 1,
            _ => {}
        }
    }
    depth > 0
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
                Some((word, _)) => return Err(unexpected(word)),
                None => {
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
        if state.loop_signal.is_some() {
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
    if *last == CommandResult::Exit {
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
