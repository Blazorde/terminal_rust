//! `Tab` completion and history hints of the line editor.

mod files;
mod git;
mod line;
mod listing;
mod matching;
mod options;
mod programs;
mod quoting;
#[cfg(test)]
mod tests;
mod variables;

use std::borrow::Cow;
use std::path::PathBuf;

use crate::utils::shell_env;
use files::Folders;
use matching::{CASES, Case};
use programs::Programs;
use quoting::quoted;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::line_buffer::LineBuffer;
use rustyline::validate::Validator;
use rustyline::{Changeset, Context, Helper};
use variables::{variable_completions, variable_reference};

pub struct ShellHelper {
    history_hinter: HistoryHinter,
    command_names: Vec<String>,
    search_path: bool,
    programs: Programs,
}

impl ShellHelper {
    pub fn with_command_names(command_names: Vec<String>) -> Self {
        Self {
            history_hinter: HistoryHinter::new(),
            command_names,
            search_path: true,
            programs: Programs::default(),
        }
    }

    /// Do not offer the programs of `PATH`: for tests that must not depend on
    /// what is installed.
    #[cfg(test)]
    fn without_path_programs(mut self) -> Self {
        self.search_path = false;
        self
    }

    /// Every candidate for the word before `pos`, and where that word starts.
    fn candidates(&self, line: &str, pos: usize) -> (usize, Vec<Pair>) {
        let at = line::analyze(&line[..pos]);
        let (command_start, current_word) = (at.start, at.word);

        if let Some((offset, prefix, braced)) = variable_reference(current_word) {
            let names: Vec<String> = shell_env::vars()
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            return (
                command_start + offset,
                variable_completions(&names, prefix, braced),
            );
        }

        let looks_like_a_path =
            current_word.contains(['/', '\\']) || current_word.starts_with(['.', '~']);
        if at.command_position && !looks_like_a_path {
            let candidates = CASES
                .into_iter()
                .map(|case| self.commands(current_word, case))
                .find(|candidates| !candidates.is_empty())
                .unwrap_or_default();
            return (command_start, candidates);
        }

        let Ok(current) = std::env::current_dir() else {
            return (command_start, Vec::new());
        };
        let home = shell_env::value("HOME")
            .or_else(|| shell_env::value("USERPROFILE"))
            .map(PathBuf::from);
        let folders = Folders {
            current: &current,
            home: home.as_deref(),
        };
        // The file of a redirection is not an argument of the command.
        let command: &[&str] = if at.redirect { &[] } else { &at.command };
        let directories_only = matches!(command.first(), Some(&("cd" | "pushd" | "rmdir")));
        if let Some(candidates) = options::complete(command, current_word) {
            return (command_start, candidates);
        }
        for case in CASES {
            // `git ch<Tab>` and `git checkout fe<Tab>`: subcommands and references.
            if let Some(candidates) = git::complete(command, current_word, &current, case) {
                return (command_start, candidates);
            }
            let (offset, candidates) =
                files::complete(current_word, case, directories_only, &folders);
            if !candidates.is_empty() {
                return (command_start + offset, candidates);
            }
        }
        (command_start, Vec::new())
    }

    /// The commands and programs that start with `prefix`.
    fn commands(&self, prefix: &str, case: Case) -> Vec<Pair> {
        let mut candidates = command_completions(&self.command_names, prefix, case);
        // Without a prefix every program of `PATH` would be listed.
        if !prefix.is_empty() {
            for program in self.programs_starting_with(prefix, case) {
                if !self.command_names.contains(&program)
                    && let Some(written) = quoted(&program, None, cfg!(windows))
                {
                    candidates.push(Pair {
                        replacement: format!("{written} "),
                        display: program,
                    });
                }
            }
        }
        candidates
    }

    fn programs_starting_with(&self, prefix: &str, case: Case) -> Vec<String> {
        if !self.search_path {
            return Vec::new();
        }
        self.programs.starting_with(prefix, case)
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, candidates) = self.candidates(line, floor_char_boundary(line, pos));
        Ok((start, listing::finish(candidates)))
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, changes: &mut Changeset) {
        let end = line.pos();
        line.replace(
            start..end,
            listing::without_dangling_escape(elected),
            changes,
        );
    }
}

/// The largest index not after `pos` that falls between two characters.
fn floor_char_boundary(text: &str, pos: usize) -> usize {
    let mut index = pos.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn command_completions(command_names: &[String], prefix: &str, case: Case) -> Vec<Pair> {
    command_names
        .iter()
        .filter(|command| case.starts_with(command, prefix))
        .map(|command| Pair {
            display: command.clone(),
            replacement: format!("{command} "),
        })
        .collect()
}

impl Hinter for ShellHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, context: &Context<'_>) -> Option<Self::Hint> {
        self.history_hinter
            .hint(line, pos, context)
            .filter(|hint| !hint.chars().any(char::is_control))
    }
}

impl Highlighter for ShellHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\u{1b}[2;90m{hint}\u{1b}[0m"))
    }
}

impl Validator for ShellHelper {}

impl Helper for ShellHelper {}
