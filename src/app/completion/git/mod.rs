//! Tab completion for `git` (see ADR-0019): the subcommands, then the branches,
//! remote branches and tags of the repository for the commands that take one.
//!
//! The references are read from the repository files (`refs/` and
//! `packed-refs`), so completing never starts Git and stays instant.
//!
//! After `git -C <dir>` everything that depends on a directory is read from that
//! one, and from none when it cannot be known before the line runs.

mod arguments;
#[cfg(test)]
mod directory_tests;
mod references;
mod repository;
#[cfg(test)]
mod test_repository;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use rustyline::completion::Pair;

use super::files::{self, Folders};
use super::matching::Case;
use super::quoting::quoted;
use arguments::{Argument, Words};

/// The everyday subcommands, offered after `git `.
const SUBCOMMANDS: [&str; 41] = [
    "add",
    "apply",
    "archive",
    "bisect",
    "blame",
    "branch",
    "checkout",
    "cherry-pick",
    "clean",
    "clone",
    "commit",
    "config",
    "describe",
    "diff",
    "fetch",
    "gc",
    "grep",
    "help",
    "init",
    "log",
    "ls-files",
    "merge",
    "mv",
    "pull",
    "push",
    "rebase",
    "reflog",
    "remote",
    "reset",
    "restore",
    "revert",
    "rm",
    "shortlog",
    "show",
    "stash",
    "status",
    "submodule",
    "switch",
    "tag",
    "version",
    "worktree",
];

/// A command and where it runs, which `git -C <dir>` changes.
pub(super) struct Invocation<'line, 'folders> {
    words: Words<'line>,
    /// `None` when it cannot be known before the line runs.
    directory: Option<PathBuf>,
    home: Option<&'folders Path>,
}

impl<'line, 'folders> Invocation<'line, 'folders> {
    /// `command` holds the words before the one being typed.
    pub(super) fn read(command: &[&'line str], folders: &Folders<'folders>) -> Self {
        let words = arguments::set_directories_aside(command);
        let home = folders.home.and_then(Path::to_str);
        let directory = words
            .directories
            .iter()
            .try_fold(folders.current.to_path_buf(), |from, typed| {
                repository::enter(&from, &arguments::literal(typed, home)?)
            });
        Self {
            words,
            directory,
            home: folders.home,
        }
    }

    /// The words of the command without its `-C <dir>` pairs.
    pub(super) fn words(&self) -> &[&'line str] {
        &self.words.command
    }

    /// Where the file names given to the command start from. `None` when none are
    /// to be offered.
    pub(super) fn folders(&self) -> Option<Folders<'_>> {
        if self.words.directory_next {
            return None;
        }
        Some(Folders {
            current: self.directory.as_deref()?,
            home: self.home,
        })
    }

    /// Candidates for `word`, the word being typed, and where in it they go, when
    /// this is a `git` command and there is something to offer. `None` leaves the
    /// completion to the file names.
    pub(super) fn complete(&self, word: &str, case: Case) -> Option<(usize, Vec<Pair>)> {
        let (start, matches) = match arguments::at_cursor(&self.words, word)? {
            // `git -C <TAB>`: the directories.
            Argument::Directory => {
                let home = self.home.and_then(Path::to_str);
                if !arguments::has_literal_directory(word, home, cfg!(windows)) {
                    return None;
                }
                let folders = Folders {
                    current: self.directory.as_deref()?,
                    home: self.home,
                };
                let (start, mut matches) = files::directories(word, case, &folders);
                matches.retain(|pair| !pair.display.starts_with('-'));
                (start, matches)
            }
            // `git <TAB>`: the subcommands.
            Argument::Subcommand => (0, subcommands(word, case)),
            // `git checkout <TAB>`: the references that match. A word that matches
            // none is a file name for the normal completion.
            Argument::Reference => (0, references(self.directory.as_deref()?, word, case)),
        };
        (!matches.is_empty()).then_some((start, matches))
    }
}

fn subcommands(word: &str, case: Case) -> Vec<Pair> {
    SUBCOMMANDS
        .iter()
        .filter(|name| case.starts_with(name, word))
        .map(|name| Pair {
            display: (*name).to_string(),
            replacement: format!("{name} "),
        })
        .collect()
}

fn references(directory: &Path, word: &str, case: Case) -> Vec<Pair> {
    references::list(directory)
        .into_iter()
        .filter(|name| case.starts_with(name, word))
        .filter_map(|name| {
            let written = quoted(&name, None, cfg!(windows))?;
            Some(Pair {
                replacement: format!("{written} "),
                display: name,
            })
        })
        .collect()
}
