//! `Tab` completion and history hints of the line editor.

mod git;
mod line;
mod listing;
mod programs;
#[cfg(test)]
mod tests;
mod variables;

use std::borrow::Cow;

use crate::utils::shell_env;
use programs::Programs;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::line_buffer::LineBuffer;
use rustyline::validate::Validator;
use rustyline::{Changeset, Context, Helper};
use variables::{variable_completions, variable_reference};

pub struct ShellHelper {
    filename_completer: FilenameCompleter,
    history_hinter: HistoryHinter,
    command_names: Vec<String>,
    search_path: bool,
    programs: Programs,
}

impl ShellHelper {
    pub fn with_command_names(command_names: Vec<String>) -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
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
    fn candidates(
        &self,
        line: &str,
        pos: usize,
        context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let at = line::analyze(&line[..pos]);
        let (command_start, current_word) = (at.start, at.word);

        if let Some((offset, prefix, braced)) = variable_reference(current_word) {
            let names: Vec<String> = shell_env::vars()
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            return Ok((
                command_start + offset,
                variable_completions(&names, prefix, braced),
            ));
        }

        let looks_like_a_path =
            current_word.contains(['/', '\\']) || current_word.starts_with(['.', '~']);
        if at.command_position && !looks_like_a_path {
            let mut candidates = command_completions(&self.command_names, current_word);
            // Without a prefix every program of `PATH` would be listed.
            if !current_word.is_empty() {
                for program in self.programs_starting_with(current_word) {
                    if !self.command_names.contains(&program) {
                        candidates.push(Pair {
                            replacement: format!("{program} "),
                            display: program,
                        });
                    }
                }
            }
            return Ok((command_start, candidates));
        }

        // `git ch<Tab>` and `git checkout fe<Tab>`: subcommands and references.
        if let Ok(directory) = std::env::current_dir()
            && let Some(candidates) = git::complete(&at.command, current_word, &directory)
        {
            return Ok((command_start, candidates));
        }

        let (start, mut candidates) = self.filename_completer.complete(line, pos, context)?;
        // Hidden files are offered only when the name being typed starts with a dot.
        let typed_name = line[start..pos]
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default();
        if !typed_name.starts_with('.') {
            candidates.retain(|candidate| !candidate.display.starts_with('.'));
        }
        Ok((start, candidates))
    }

    fn programs_starting_with(&self, prefix: &str) -> Vec<String> {
        if !self.search_path {
            return Vec::new();
        }
        self.programs.starting_with(prefix)
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, candidates) = self.candidates(line, floor_char_boundary(line, pos), context)?;
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

fn command_completions(command_names: &[String], prefix: &str) -> Vec<Pair> {
    command_names
        .iter()
        .filter(|command| command.starts_with(prefix))
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
