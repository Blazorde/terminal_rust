//! Finding the repository around a directory and reading its files.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Add the files under `directory`, named relative to it (`feature/x`), reading
/// at most `limit` of them.
pub(super) fn collect_loose(directory: &Path, names: &mut BTreeSet<String>, limit: usize) {
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
    let mut budget = limit;
    walk(directory, directory, names, &mut budget);
}

/// The directory `git -C <value>` moves to when run in `from`. `None` when Git
/// could not move there.
pub(super) fn enter(from: &Path, value: &str) -> Option<PathBuf> {
    if value.is_empty() {
        return Some(from.to_path_buf());
    }
    let target = from.join(value);
    // Git changes directory, so `..` after a link leaves the target of the link, and
    // the walk up to the repository must start there too. Windows instead removes
    // `..` from the text of a path.
    let target = if cfg!(windows) {
        std::path::absolute(target)
    } else {
        fs::canonicalize(target)
    };
    target.ok().filter(|target| target.is_dir())
}

/// The directory that holds the `refs` of the repository around `directory`: its
/// `.git`, or for a linked worktree the shared directory named by `commondir`.
pub(super) fn locate(directory: &Path) -> Option<PathBuf> {
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
