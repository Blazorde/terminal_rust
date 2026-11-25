//! The files and directories that can complete a word.

use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use rustyline::completion::Pair;

use super::matching::Case;
use super::quoting::{PathWord, quoted, split_path};

/// Where relative paths and `~` start.
pub(super) struct Folders<'a> {
    pub(super) current: &'a Path,
    pub(super) home: Option<&'a Path>,
}

impl Folders<'_> {
    fn locate(&self, path: &PathWord<'_>) -> PathBuf {
        if path.home
            && let Some(home) = self.home
        {
            let rest = path.directory[1..].trim_start_matches(['/', '\\']);
            return home.join(rest);
        }
        self.current.join(&path.directory)
    }
}

/// The names that can complete the path in `word`, and where in the word they go.
/// Hidden files are offered only when the name typed starts with a dot.
pub(super) fn complete(word: &str, case: Case, folders: &Folders<'_>) -> (usize, Vec<Pair>) {
    let path = split_path(word, cfg!(windows));
    let Ok(entries) = fs::read_dir(folders.locate(&path)) else {
        return (path.start, Vec::new());
    };
    let separator = path.separator.unwrap_or(MAIN_SEPARATOR);
    let candidates = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let hidden = name.starts_with('.') && !path.name.starts_with('.');
            if hidden || !case.starts_with(&name, &path.name) {
                return None;
            }
            // `metadata` follows a link to a directory.
            let is_directory = fs::metadata(entry.path()).is_ok_and(|found| found.is_dir());
            candidate(&path, name, is_directory, separator)
        })
        .collect();
    (path.start, candidates)
}

fn candidate(
    path: &PathWord<'_>,
    mut name: String,
    is_directory: bool,
    separator: char,
) -> Option<Pair> {
    let mut replacement = format!("{}{}", path.kept, quoted(&name, path.quote, cfg!(windows))?);
    if is_directory {
        replacement.push(separator);
        name.push(MAIN_SEPARATOR);
    } else if let Some(quote) = path.quote {
        replacement.push(quote);
    }
    Some(Pair {
        display: name,
        replacement,
    })
}
