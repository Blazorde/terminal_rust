//! The entries of the stash, named by their place: `stash@{0}` is the latest.
//! Their messages are not shown.

use std::path::Path;

use rustyline::completion::Pair;

use super::matching_names;
use super::repository::{locate, read_tail};
use crate::app::completion::matching::Case;
use crate::app::completion::quoting::quoted;

/// The most bytes read from the end of the log of the stash.
pub(super) const MAX_LOG_BYTES: usize = 256 * 1024;

/// The most entries offered, the latest ones.
pub(super) const MAX_ENTRIES: usize = 100;

/// The names of the entries of the repository that contains `directory`, the
/// latest first. Empty outside a repository.
pub(super) fn list(directory: &Path) -> Vec<String> {
    let log = locate(directory)
        .and_then(|common| read_tail(&common.join("logs/refs/stash"), MAX_LOG_BYTES));
    let Some(log) = log else {
        return Vec::new();
    };
    // Git writes one line per entry, the latest last, and ignores one that is not
    // ended. Counted on bytes: a message that is not text must not shift the places.
    let entries = log.iter().filter(|&&byte| byte == b'\n').count();
    (0..entries.min(MAX_ENTRIES))
        .map(|place| format!("stash@{{{place}}}"))
        .collect()
}

/// The entries that start with `word`. They are inserted quoted, for their braces,
/// so what two of them share opens a quote: such a word still matches, keeps its
/// quote and is closed.
pub(super) fn matching(names: Vec<String>, word: &str, case: Case) -> Vec<Pair> {
    let Some(quote) = word
        .chars()
        .next()
        .filter(|first| matches!(first, '\'' | '"'))
    else {
        return matching_names(names, word, case);
    };
    let typed = &word[quote.len_utf8()..];
    names
        .into_iter()
        .filter(|name| case.starts_with(name, typed))
        .filter_map(|name| {
            let written = quoted(&name, Some(quote), cfg!(windows))?;
            Some(Pair {
                replacement: format!("{quote}{written}{quote} "),
                display: name,
            })
        })
        .collect()
}
