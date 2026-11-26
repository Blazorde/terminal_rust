//! What goes at the cursor of a `git` command, read from its words alone.

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
    Subcommand,
    /// A branch, a remote branch or a tag.
    Reference,
}

/// The argument expected for `word`, which follows `words` in its command.
/// `None` when this is not a `git` command or there is nothing to offer there.
pub(super) fn at_cursor(words: &[&str], word: &str) -> Option<Argument> {
    if word.starts_with('-') {
        return None;
    }
    match words {
        ["git"] => Some(Argument::Subcommand),
        ["git", subcommand, ..] if REFERENCE_SUBCOMMANDS.contains(subcommand) => {
            Some(Argument::Reference)
        }
        _ => None,
    }
}
