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
pub(super) fn complete(
    word: &str,
    case: Case,
    directories_only: bool,
    folders: &Folders<'_>,
) -> (usize, Vec<Pair>) {
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
            if directories_only && !is_directory {
                return None;
            }
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

#[cfg(test)]
mod tests {
    use super::super::matching::CASES;
    use super::*;
    use crate::utils::path::temp_path;

    /// An empty directory of its own, with these files and directories (`/` at the end).
    fn fixture(name: &str, entries: &[&str]) -> PathBuf {
        let root = temp_path("completion", name);
        fs::create_dir_all(&root).expect("fixture");
        for entry in entries {
            match entry.strip_suffix('/') {
                Some(directory) => fs::create_dir_all(root.join(directory)).expect("fixture"),
                None => fs::write(root.join(entry), "x").expect("fixture"),
            }
        }
        root
    }

    /// The replacements for `word` in `root`, sorted: the exact case first.
    fn paths_in(root: &Path, word: &str) -> Vec<String> {
        let folders = Folders {
            current: root,
            home: Some(root),
        };
        let found = CASES
            .into_iter()
            .map(|case| complete(word, case, false, &folders).1)
            .find(|candidates| !candidates.is_empty())
            .unwrap_or_default();
        let mut names: Vec<String> = found.into_iter().map(|pair| pair.replacement).collect();
        names.sort();
        names
    }

    #[test]
    fn another_case_is_offered_only_without_the_one_typed() {
        // Two names that differ by more than their case: some systems ignore it.
        let root = fixture("case", &["Readme.md", "reading-list"]);

        assert_eq!(paths_in(&root, "read"), ["reading-list"]);
        assert_eq!(paths_in(&root, "Read"), ["Readme.md"]);
        // No name starts with `READ`: both are offered, in their own case.
        assert_eq!(paths_in(&root, "READ"), ["Readme.md", "reading-list"]);
        assert_eq!(paths_in(&root, "readm"), ["Readme.md"]);
        assert!(paths_in(&root, "zz").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_name_with_special_characters_is_written_for_the_shell() {
        let root = fixture("quoting", &["my notes.txt", "my dir/"]);
        let separator = MAIN_SEPARATOR;
        let spaced = |name: &str| {
            if cfg!(windows) {
                format!("'{name}'")
            } else {
                name.replace(' ', "\\ ")
            }
        };

        assert_eq!(
            paths_in(&root, "my"),
            [
                format!("{}{separator}", spaced("my dir")),
                spaced("my notes.txt")
            ]
        );
        // Inside quotes the name stays as it is, and a file closes them.
        assert_eq!(paths_in(&root, "\"my n"), ["my notes.txt\""]);
        assert_eq!(paths_in(&root, "'my d"), [format!("my dir{separator}")]);
        assert_eq!(
            paths_in(&root, &format!("~{separator}my n")),
            [format!("~{separator}{}", spaced("my notes.txt"))]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
