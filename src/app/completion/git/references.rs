//! The branches, remote branches and tags of a repository.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::repository::{
    collect_loose, locate, locate_worktree, read_head, read_pointer, text_lines,
};

/// The most references read from a repository: a huge one must not make the
/// `Tab` key slow.
const MAX_REFERENCES: usize = 5_000;

/// The most bytes read from `packed-refs`.
pub(super) const MAX_PACKED_BYTES: usize = 8 * 1024 * 1024;

const HEADS: &str = "refs/heads";
const REMOTES: &str = "refs/remotes";
const TAGS: &str = "refs/tags";

/// The branches, remote branches and tags of the repository that contains
/// `directory`, sorted. Empty outside a repository.
pub(super) fn list(directory: &Path) -> Vec<String> {
    offered(read(directory, &[HEADS, REMOTES, TAGS]))
}

/// The local branches that can be deleted from `directory`: all of them but the one
/// checked out in its working tree, sorted.
pub(super) fn local_branches(directory: &Path) -> Vec<String> {
    let mut names = read(directory, &[HEADS]);
    if let Some(current) = checked_out(directory) {
        let ignore_case = cfg!(any(windows, target_os = "macos"));
        remove_current(&mut names, &current, ignore_case);
    }
    offered(names)
}

/// The remote branches of the repository that contains `directory`, sorted.
pub(super) fn remote_branches(directory: &Path) -> Vec<String> {
    offered(read(directory, &[REMOTES]))
}

/// The branch checked out in the working tree around `directory`. `None` when its
/// `HEAD` is detached.
fn checked_out(directory: &Path) -> Option<String> {
    let head = locate_worktree(directory)?.join("HEAD");
    let target = match fs::read_link(&head) {
        // `core.preferSymlinkRefs`: following the link would read a commit, or nothing
        // once the ref is packed. A ref never holds `\`, which Windows may put there.
        Ok(link) => link.to_str()?.replace('\\', "/"),
        Err(_) => read_pointer(&head)?
            .trim()
            .strip_prefix("ref:")?
            .trim()
            .to_string(),
    };
    target.strip_prefix("refs/heads/").map(str::to_string)
}

/// Take the checked out branch out of `names`. Where file names ignore case, `HEAD`
/// keeps the case that was typed to check it out, which may not be that of the ref.
pub(super) fn remove_current(names: &mut BTreeSet<String>, current: &str, ignore_case: bool) {
    if ignore_case {
        let current = current.to_lowercase();
        names.retain(|name| name.to_lowercase() != current);
    } else {
        names.remove(current);
    }
}

/// The names under `namespaces`, loose and packed, each without its namespace.
fn read(directory: &Path, namespaces: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(common) = locate(directory) else {
        return names;
    };
    for namespace in namespaces {
        collect_loose(&common.join(namespace), &mut names, MAX_REFERENCES);
    }
    if let Some(packed) = read_head(&common.join("packed-refs"), MAX_PACKED_BYTES) {
        for line in text_lines(&packed) {
            // `<sha> refs/heads/name`; lines starting with `#` or `^` are not refs.
            if let Some((_, name)) = line.split_once(' ') {
                for namespace in namespaces {
                    if let Some(short) = name
                        .strip_prefix(namespace)
                        .and_then(|rest| rest.strip_prefix('/'))
                    {
                        names.insert(short.to_string());
                    }
                }
            }
        }
    }
    names
}

fn offered(names: BTreeSet<String>) -> Vec<String> {
    names
        .into_iter()
        // Git would read a name that starts with `-` as an option.
        .filter(|name| name != "HEAD" && !name.ends_with("/HEAD") && !name.starts_with('-'))
        .take(MAX_REFERENCES)
        .collect()
}
