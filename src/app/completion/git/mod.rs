//! Tab completion for `git` (see ADR-0019): the subcommands, then the branches,
//! remote branches and tags of the repository for the commands that take one.
//!
//! The references are read from the repository files (`refs/` and
//! `packed-refs`), so completing never starts Git and stays instant.

mod arguments;
mod references;
mod repository;
#[cfg(test)]
mod tests;

use std::path::Path;

use rustyline::completion::Pair;

use super::matching::Case;
use super::quoting::quoted;
use arguments::Argument;

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

/// Candidates for the word being typed, when the line is a `git` command and
/// there is something to offer. `None` leaves the completion to the file names.
///
/// `words` are those of the command before the word, `word` the word itself, and
/// `directory` where the repository is looked for.
pub(super) fn complete(
    words: &[&str],
    word: &str,
    directory: &Path,
    case: Case,
) -> Option<Vec<Pair>> {
    let matches: Vec<Pair> = match arguments::at_cursor(words, word)? {
        // `git <TAB>`: the subcommands.
        Argument::Subcommand => SUBCOMMANDS
            .iter()
            .filter(|name| case.starts_with(name, word))
            .map(|name| Pair {
                display: (*name).to_string(),
                replacement: format!("{name} "),
            })
            .collect(),
        // `git checkout <TAB>`: the references that match. A word that matches
        // none is a file name for the normal completion.
        Argument::Reference => references::list(directory)
            .into_iter()
            .filter(|name| case.starts_with(name, word))
            .filter_map(|name| {
                let written = quoted(&name, None, cfg!(windows))?;
                Some(Pair {
                    replacement: format!("{written} "),
                    display: name,
                })
            })
            .collect(),
    };
    (!matches.is_empty()).then_some(matches)
}
