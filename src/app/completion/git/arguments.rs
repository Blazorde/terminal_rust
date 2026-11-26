//! What goes at the cursor of a `git` command, and where it runs, read from its
//! words alone.

use crate::app::completion::quoting::split_whole_path;
use crate::app::history;
use crate::app::parser::parse_words;

/// The subcommands whose arguments are often a branch, a remote branch or a tag.
const REFERENCE_SUBCOMMANDS: [&str; 14] = [
    "checkout",
    "switch",
    "merge",
    "rebase",
    "branch",
    "cherry-pick",
    "diff",
    "log",
    "show",
    "reset",
    "revert",
    "push",
    "pull",
    "fetch",
];

/// A kind of argument Git completion knows how to offer.
pub(super) enum Argument {
    /// The value of `-C`.
    Directory,
    Subcommand,
    /// A branch, a remote branch or a tag.
    Reference,
}

/// The words of a command, the `-C <dir>` pairs of `git` set aside.
pub(super) struct Words<'a> {
    pub(super) command: Vec<&'a str>,
    /// The values of those pairs, as typed.
    pub(super) directories: Vec<&'a str>,
    /// Whether the word at the cursor is the value of a `-C`.
    pub(super) directory_next: bool,
}

/// Only the pairs before the subcommand count: `git commit -C` is something else.
/// Git does not accept the value attached, as in `-Cdir`.
pub(super) fn set_directories_aside<'a>(command: &[&'a str]) -> Words<'a> {
    let mut words = Words {
        command: command.to_vec(),
        directories: Vec::new(),
        directory_next: false,
    };
    let ["git", after @ ..] = command else {
        return words;
    };
    let mut rest = after;
    while let ["-C", tail @ ..] = rest {
        let Some((value, tail)) = tail.split_first() else {
            words.directory_next = true;
            rest = tail;
            break;
        };
        words.directories.push(value);
        rest = tail;
    }
    words.command.truncate(1);
    words.command.extend_from_slice(rest);
    words
}

/// The text the shell would make of `typed`, a word of the line. `None` when it
/// cannot be known without running the line: nothing is evaluated here.
pub(super) fn literal(typed: &str, home: Option<&str>) -> Option<String> {
    // Variables, substitutions, braces and history references rewrite the word.
    if typed.contains(['$', '`', '{']) || history::expand(typed, &[]) != Ok(None) {
        return None;
    }
    let lookup = |name: &str| home.filter(|_| name == "HOME").map(str::to_string);
    let mut words = parse_words(typed, &lookup).ok()?;
    let word = words.pop()?;
    (words.is_empty() && word.pattern.is_none()).then_some(word.text)
}

/// Whether the directory part of `word`, a path being typed, is the one the shell
/// would read there. The name after it is only matched against the disk, then
/// written back quoted.
pub(super) fn has_literal_directory(word: &str, home: Option<&str>, windows: bool) -> bool {
    let path = split_whole_path(word, windows);
    let mut typed = word[..path.start + path.kept.len()].to_string();
    if typed.is_empty() {
        return true;
    }
    // A last Windows `\` would escape the quote added here, and `/` separates too.
    if path.quote == Some('"') && typed.ends_with('\\') {
        typed.pop();
        typed.push('/');
    }
    typed.extend(path.quote);
    literal(&typed, home).is_some()
}

/// The argument expected for `word`, which follows `words` in its command.
/// `None` when this is not a `git` command or there is nothing to offer there.
pub(super) fn at_cursor(words: &Words<'_>, word: &str) -> Option<Argument> {
    if words.directory_next {
        return Some(Argument::Directory);
    }
    if word.starts_with('-') {
        return None;
    }
    match words.command.as_slice() {
        ["git"] => Some(Argument::Subcommand),
        ["git", subcommand, ..] if REFERENCE_SUBCOMMANDS.contains(subcommand) => {
            Some(Argument::Reference)
        }
        _ => None,
    }
}
