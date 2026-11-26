//! Tab completion for `git` (see ADR-0019): the subcommands, then for the
//! commands that take one the branches, remote branches and tags of the
//! repository, the names of its remotes or the entries of its stash. When `git
//! branch` is asked to delete, only the branches of the kind it deletes.
//!
//! They are read from the repository files (`refs/`, `packed-refs`, `HEAD`,
//! `config` and the log of the stash), so completing never starts Git and stays
//! instant.
//!
//! After `git -C <dir>` everything that depends on a directory is read from that
//! one, and from none when it cannot be known before the line runs.

mod arguments;
#[cfg(test)]
mod branch_tests;
#[cfg(test)]
mod directory_tests;
#[cfg(test)]
mod file_reading_tests;
mod references;
#[cfg(test)]
mod remote_tests;
mod remotes;
mod repository;
mod stash;
#[cfg(test)]
mod stash_tests;
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
            Argument::Reference => {
                let names = references::list(self.directory.as_deref()?);
                (0, matching_names(names, word, case))
            }
            // `git branch -d <TAB>`: only what can be deleted.
            Argument::LocalBranch => {
                let names = references::local_branches(self.directory.as_deref()?);
                (0, matching_names(names, word, case))
            }
            Argument::RemoteBranch => {
                let names = references::remote_branches(self.directory.as_deref()?);
                (0, matching_names(names, word, case))
            }
            // `git push <TAB>`: the remotes. A path is a repository too, left to
            // the file names.
            Argument::Remote => {
                let names = remotes::list(self.directory.as_deref()?);
                (0, matching_names(names, word, case))
            }
            // `git stash pop <TAB>`: the entries.
            Argument::Stash => {
                let names = stash::list(self.directory.as_deref()?);
                (0, stash::matching(names, word, case))
            }
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

/// The names read from the repository that start with `word`, written for the
/// shell.
fn matching_names(names: Vec<String>, word: &str, case: Case) -> Vec<Pair> {
    names
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
