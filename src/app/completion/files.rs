//! The files and directories that can complete a word.

use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use rustyline::completion::Pair;

use super::matching::Case;
use super::quoting::{PathWord, quoted, split_path, split_whole_path};

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
    let candidates = in_directory(&path, case, directories_only, folders);
    // `@file` names a file for `curl` and others.
    if candidates.is_empty()
        && let Some(rest) = word.strip_prefix('@')
    {
        let (start, candidates) = complete(rest, case, directories_only, folders);
        return (start + 1, candidates);
    }
    (path.start, candidates)
}

/// The directories that can complete `word`, which is a path as a whole: neither
/// a `=` nor a `@` introduces one in it.
pub(super) fn directories(word: &str, case: Case, folders: &Folders<'_>) -> (usize, Vec<Pair>) {
    let path = split_whole_path(word, cfg!(windows));
    (path.start, in_directory(&path, case, true, folders))
}

fn in_directory(
    path: &PathWord<'_>,
    case: Case,
    directories_only: bool,
    folders: &Folders<'_>,
) -> Vec<Pair> {
    let Ok(entries) = fs::read_dir(folders.locate(path)) else {
        return Vec::new();
    };
    let separator = path.separator.unwrap_or(MAIN_SEPARATOR);
    entries
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
            candidate(path, name, is_directory, separator, cfg!(windows))
        })
        .collect()
}

fn candidate(
    path: &PathWord<'_>,
    mut name: String,
    is_directory: bool,
    separator: char,
    windows: bool,
) -> Option<Pair> {
    let written = quoted(&name, path.quote, windows)?;
    let mut replacement = path.kept.to_string();
    // A last `\` would escape a quote or a `\` that starts the name, and what the name
    // says would then run. Windows reads `/` as a separator too.
    if path.quote != Some('\'') && written.starts_with(['\'', '\\']) && replacement.ends_with('\\')
    {
        replacement.pop();
        replacement.push('/');
    }
    replacement.push_str(&written);
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
    use crate::ShellEngine;
    use crate::app::history;
    use crate::app::parser::{parse_words, split_chain};
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
    fn a_file_named_after_an_at_sign_completes() {
        let root = fixture("at", &["payload.json", "@types/"]);
        let separator = MAIN_SEPARATOR;
        let folders = Folders {
            current: &root,
            home: None,
        };
        let (start, found) = complete("@pay", Case::Exact, false, &folders);

        assert_eq!((start, found[0].replacement.as_str()), (1, "payload.json"));
        // A name that does start with `@` comes first.
        assert_eq!(paths_in(&root, "@ty"), [format!("@types{separator}")]);
        fs::remove_dir_all(root).expect("cleanup");
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
        // On Windows the name opens a quote, which a `\` in front would escape.
        assert_eq!(
            paths_in(&root, &format!("~{separator}my n")),
            [format!("~/{}", spaced("my notes.txt"))]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    /// `split_path` or `split_whole_path`.
    type Split = fn(&str, bool) -> PathWord<'_>;

    /// The line `echo <word>` once the Windows completion has put `name` in `word`.
    /// `None` when the name is not offered there.
    fn windows_line(split: Split, word: &str, name: &str) -> Option<String> {
        let path = split(word, true);
        let pair = candidate(&path, name.to_string(), false, '\\', true)?;
        Some(format!("echo {}{}", &word[..path.start], pair.replacement))
    }

    #[test]
    fn a_windows_path_never_lets_a_name_act_on_the_line() {
        let names = [
            "$(echo INJ)",
            "$x",
            ";echo INJ;'",
            "'; echo INJ",
            "a b",
            "`echo INJ`",
            "a!b",
            "a&echo INJ",
            "plain.txt",
        ];
        let lookup = |name: &str| (name == "HOME").then(|| "HOMEDIR".to_string());
        let mut engine = ShellEngine::new();

        let splits: [Split; 2] = [split_path, split_whole_path];
        let directories = ["dir\\", "~\\", "a\\dir\\", "'a b'\\", "a=b\\"];
        for (split, directory) in splits
            .into_iter()
            .flat_map(|split| directories.map(|directory| (split, directory)))
        {
            for quote in ["", "\""] {
                // A quote after a quoted directory is another word shape.
                if quote == "\"" && directory.starts_with('\'') {
                    continue;
                }
                for name in names {
                    let word = format!("{quote}{directory}");
                    let Some(line) = windows_line(split, &word, name) else {
                        assert!(quote == "\"" && name.contains('`'), "{word} {name}");
                        continue;
                    };
                    let typed = directory.replace('\'', "").replace('\\', "/");
                    let expected = match typed.strip_prefix('~') {
                        Some(rest) if quote.is_empty() => format!("HOMEDIR{rest}{name}"),
                        _ => format!("{typed}{name}"),
                    };

                    let segments = split_chain(&line).expect(&line);
                    assert_eq!(segments.len(), 1, "{line}");
                    assert!(!segments[0].background, "{line}");
                    let words = parse_words(&segments[0].text, &lookup).expect(&line);
                    assert_eq!(words.len(), 2, "{line}");
                    assert_eq!(words[1].pattern, None, "{line}");
                    let read = words[1].text.replace('\\', "/");
                    assert_eq!(read, expected, "{line}");

                    // The parser alone does not run substitutions: the shell does.
                    let result = engine.execute_line(&line);
                    assert_eq!(result.stdout.lines().count(), 1, "{line}");
                    let printed = result.stdout.trim_end();
                    assert!(printed.ends_with(name), "{line} printed {printed}");
                }
            }
        }
    }

    #[test]
    fn a_completed_name_is_never_expanded_by_the_history() {
        let history = ["echo INJ".to_string()];
        for windows in [false, true] {
            for word in ["", "'", "\"", "dir/", "'dir/", "\"dir/"] {
                for name in ["a!!", "!!", "a\"!!", "a'!!", "a b!$", "$!!", "\"'!1"] {
                    let path = split_path(word, windows);
                    let pair = candidate(&path, name.to_string(), false, '/', windows);
                    let pair = pair.expect("a name without a backquote is offered");
                    let line = format!("cat {}{}", &word[..path.start], pair.replacement);

                    assert_eq!(history::expand(&line, &history), Ok(None), "{line}");
                }
            }
        }
    }
}
