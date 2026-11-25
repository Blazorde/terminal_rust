//! Tab completion for `git` (see ADR-0019): the subcommands, then the branches,
//! remote branches and tags of the repository for the commands that take one.
//!
//! The references are read from the repository files (`refs/` and
//! `packed-refs`), so completing never starts Git and stays instant.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rustyline::completion::Pair;

use super::matching::Case;
use super::quoting::quoted;

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

/// The most references read from a repository: a huge one must not make the
/// `Tab` key slow.
const MAX_REFERENCES: usize = 5_000;

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
    if words.first() != Some(&"git") || word.starts_with('-') {
        return None;
    }

    // `git <TAB>`: the subcommands.
    if words.len() == 1 {
        let matches: Vec<Pair> = SUBCOMMANDS
            .iter()
            .filter(|name| case.starts_with(name, word))
            .map(|name| Pair {
                display: (*name).to_string(),
                replacement: format!("{name} "),
            })
            .collect();
        return (!matches.is_empty()).then_some(matches);
    }

    // `git checkout <TAB>`: the references that match. A word that matches none
    // is a file name for the normal completion.
    let subcommand = words[1];
    if !REFERENCE_SUBCOMMANDS.contains(&subcommand) {
        return None;
    }
    let matches: Vec<Pair> = references(directory)
        .into_iter()
        .filter(|name| case.starts_with(name, word))
        .filter_map(|name| {
            let written = quoted(&name, None, cfg!(windows))?;
            Some(Pair {
                replacement: format!("{written} "),
                display: name,
            })
        })
        .collect();
    (!matches.is_empty()).then_some(matches)
}

/// The branches, remote branches and tags of the repository that contains
/// `directory`, sorted. Empty outside a repository.
fn references(directory: &Path) -> Vec<String> {
    let Some(common) = locate(directory) else {
        return Vec::new();
    };
    let mut names = BTreeSet::new();
    for namespace in ["refs/heads", "refs/remotes", "refs/tags"] {
        collect_loose(&common.join(namespace), &mut names);
    }
    if let Ok(packed) = fs::read_to_string(common.join("packed-refs")) {
        for line in packed.lines() {
            // `<sha> refs/heads/name`; lines starting with `#` or `^` are not refs.
            if let Some((_, name)) = line.split_once(' ') {
                for namespace in ["refs/heads/", "refs/remotes/", "refs/tags/"] {
                    if let Some(short) = name.strip_prefix(namespace) {
                        names.insert(short.to_string());
                    }
                }
            }
        }
    }
    names
        .into_iter()
        // Git would read a name that starts with `-` as an option.
        .filter(|name| name != "HEAD" && !name.ends_with("/HEAD") && !name.starts_with('-'))
        .take(MAX_REFERENCES)
        .collect()
}

/// Add the files under `directory`, named relative to it (`feature/x`).
fn collect_loose(directory: &Path, names: &mut BTreeSet<String>) {
    fn walk(root: &Path, current: &Path, names: &mut BTreeSet<String>, budget: &mut usize) {
        let Ok(entries) = fs::read_dir(current) else {
            return;
        };
        for entry in entries.flatten() {
            if *budget == 0 {
                return;
            }
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                walk(root, &path, names, budget);
            } else if file_type.is_file()
                && let Ok(relative) = path.strip_prefix(root)
            {
                *budget -= 1;
                names.insert(
                    relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    let mut budget = MAX_REFERENCES;
    walk(directory, directory, names, &mut budget);
}

/// The directory that holds the `refs` of the repository around `directory`: its
/// `.git`, or for a linked worktree the shared directory named by `commondir`.
fn locate(directory: &Path) -> Option<PathBuf> {
    let mut current = Some(directory);
    while let Some(candidate) = current {
        let dot_git = candidate.join(".git");
        if dot_git.is_dir() {
            return Some(dot_git);
        }
        if dot_git.is_file() {
            // A worktree or a submodule: `gitdir: <path>`.
            let text = fs::read_to_string(&dot_git).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            let git_dir = candidate.join(target);
            return Some(
                fs::read_to_string(git_dir.join("commondir"))
                    .ok()
                    .map_or_else(|| git_dir.clone(), |relative| git_dir.join(relative.trim())),
            );
        }
        current = candidate.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;

    /// Complete `word` typed after `before_word`.
    fn complete(before_word: &str, word: &str, directory: &Path) -> Option<Vec<Pair>> {
        let at = crate::app::completion::line::analyze(before_word);
        super::complete(&at.command, word, directory, Case::Exact)
    }

    fn displays(pairs: Option<Vec<Pair>>) -> Vec<String> {
        pairs
            .unwrap_or_default()
            .into_iter()
            .map(|pair| pair.display)
            .collect()
    }

    /// A repository with branches, a nested branch, a remote, tags and packed refs.
    fn repository(name: &str) -> PathBuf {
        let root = temp_path("gitcomp", name);
        let git = root.join(".git");
        fs::create_dir_all(git.join("refs/heads/feature")).expect("fixture");
        fs::create_dir_all(git.join("refs/remotes/origin")).expect("fixture");
        fs::create_dir_all(git.join("refs/tags")).expect("fixture");
        for file in [
            "refs/heads/main",
            "refs/heads/develop",
            "refs/heads/feature/login",
            "refs/remotes/origin/main",
            "refs/remotes/origin/HEAD",
            "refs/tags/v1.0",
        ] {
            fs::write(git.join(file), "0000000000000000000000000000000000000000\n")
                .expect("fixture");
        }
        fs::write(
            git.join("packed-refs"),
            "# pack-refs with: peeled fully-peeled sorted\n\
             1111111111111111111111111111111111111111 refs/heads/packed-branch\n\
             2222222222222222222222222222222222222222 refs/tags/v0.9\n\
             ^3333333333333333333333333333333333333333\n\
             4444444444444444444444444444444444444444 refs/remotes/origin/release\n\
             5555555555555555555555555555555555555555 refs/notes/ignored\n",
        )
        .expect("fixture");
        root
    }

    #[test]
    fn subcommands_complete_after_git() {
        let root = temp_path("gitcomp", "none");
        fs::create_dir_all(&root).expect("fixture");
        assert_eq!(
            displays(complete("git ", "ch", &root)),
            vec!["checkout", "cherry-pick"]
        );
        assert_eq!(
            displays(complete("git ", "st", &root)),
            vec!["stash", "status"]
        );
        // Every subcommand is offered when nothing is typed.
        assert_eq!(
            displays(complete("git ", "", &root)).len(),
            SUBCOMMANDS.len()
        );
        assert!(complete("git ", "zzz", &root).is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn the_subcommand_list_is_sorted_and_without_duplicates() {
        let mut sorted = SUBCOMMANDS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, SUBCOMMANDS.to_vec());
    }

    #[test]
    fn a_subcommand_is_followed_by_a_space() {
        let root = temp_path("gitcomp", "space");
        fs::create_dir_all(&root).expect("fixture");
        let pairs = complete("git ", "sta", &root).expect("matches");
        assert!(pairs.iter().all(|pair| pair.replacement.ends_with(' ')));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn branches_remote_branches_and_tags_complete_after_checkout() {
        let root = repository("refs");
        let all = displays(complete("git checkout ", "", &root));
        assert_eq!(
            all,
            vec![
                "develop",
                "feature/login",
                "main",
                "origin/main",
                "origin/release",
                "packed-branch",
                "v0.9",
                "v1.0",
            ]
        );
        assert_eq!(
            displays(complete("git switch ", "fe", &root)),
            vec!["feature/login"]
        );
        assert_eq!(
            displays(complete("git merge ", "origin/", &root)),
            vec!["origin/main", "origin/release"]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_nested_directory_of_the_repository_finds_the_references() {
        let root = repository("nested");
        let deep = root.join("src/app");
        fs::create_dir_all(&deep).expect("fixture");
        assert_eq!(
            displays(complete("git log ", "dev", &deep)),
            vec!["develop"]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn the_symbolic_head_of_a_remote_and_other_namespaces_are_not_offered() {
        let root = repository("head");
        let all = displays(complete("git checkout ", "", &root));
        assert!(!all.iter().any(|name| name.ends_with("HEAD")), "{all:?}");
        assert!(!all.iter().any(|name| name.contains("ignored")), "{all:?}");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_reference_that_reads_as_an_option_is_not_offered() {
        let root = repository("dash");
        let git = root.join(".git");
        fs::write(git.join("refs/heads/-D"), "0\n").expect("fixture");
        fs::write(git.join("packed-refs"), "1 refs/tags/--force\n").expect("fixture");
        let all = displays(complete("git checkout ", "", &root));
        assert!(all.contains(&"main".to_string()), "{all:?}");
        assert!(!all.iter().any(|name| name.starts_with('-')), "{all:?}");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_linked_worktree_reads_the_shared_references() {
        let root = repository("worktree");
        let linked = temp_path("gitcomp", "linked");
        let admin = root.join(".git/worktrees/linked");
        fs::create_dir_all(&admin).expect("fixture");
        fs::create_dir_all(&linked).expect("fixture");
        fs::write(admin.join("commondir"), "../..\n").expect("fixture");
        fs::write(
            linked.join(".git"),
            format!("gitdir: {}\n", admin.display()),
        )
        .expect("fixture");
        assert_eq!(
            displays(complete("git checkout ", "mai", &linked)),
            vec!["main"]
        );
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_dir_all(linked).expect("cleanup");
    }

    #[test]
    fn other_commands_options_and_files_are_left_to_the_file_completion() {
        let root = repository("others");
        // Not git.
        assert!(complete("echo ", "ma", &root).is_none());
        // A subcommand whose arguments are files.
        assert!(complete("git add ", "ma", &root).is_none());
        assert!(complete("git rm ", "ma", &root).is_none());
        // An option.
        assert!(complete("git checkout ", "--", &root).is_none());
        assert!(complete("git ", "--ver", &root).is_none());
        // A quoted or escaped word matches no reference: it is a file name.
        assert!(complete("git checkout ", "'ma", &root).is_none());
        assert!(complete("git checkout ", "ma\\ n", &root).is_none());
        // Nothing matches: the file names take over.
        assert!(complete("git checkout ", "src/", &root).is_none());
        // Outside a repository.
        let elsewhere = temp_path("gitcomp", "outside");
        fs::create_dir_all(&elsewhere).expect("fixture");
        assert!(complete("git checkout ", "", &elsewhere).is_none());
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_dir_all(elsewhere).expect("cleanup");
    }

    #[test]
    fn git_after_a_separator_is_recognised() {
        let root = repository("separator");
        assert_eq!(
            displays(complete("cd x && git checkout ", "dev", &root)),
            vec!["develop"]
        );
        assert_eq!(
            displays(complete("echo hi | git ", "sta", &root)),
            vec!["stash", "status"]
        );
        // The command before the separator is not git.
        assert!(complete("git status; echo ", "ma", &root).is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_missing_or_unreadable_repository_gives_nothing_without_failing() {
        let root = temp_path("gitcomp", "broken");
        fs::create_dir_all(root.join(".git")).expect("fixture");
        // A `.git` directory with no refs at all.
        assert!(complete("git checkout ", "", &root).is_none());
        // A `.git` file that points nowhere.
        let pointer = temp_path("gitcomp", "pointer");
        fs::create_dir_all(&pointer).expect("fixture");
        fs::write(pointer.join(".git"), "gitdir: /terminal-rust/no/such/dir\n").expect("fixture");
        assert!(complete("git checkout ", "", &pointer).is_none());
        fs::write(pointer.join(".git"), "garbage").expect("fixture");
        assert!(complete("git checkout ", "", &pointer).is_none());
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_dir_all(pointer).expect("cleanup");
    }
}
