//! The branches, remote branches and tags of a repository.

use std::collections::BTreeSet;
use std::path::Path;

use super::repository::{collect_loose, locate, read_head, text_lines};

/// The most references read from a repository: a huge one must not make the
/// `Tab` key slow.
const MAX_REFERENCES: usize = 5_000;

/// The most bytes read from `packed-refs`.
pub(super) const MAX_PACKED_BYTES: usize = 8 * 1024 * 1024;

/// The branches, remote branches and tags of the repository that contains
/// `directory`, sorted. Empty outside a repository.
pub(super) fn list(directory: &Path) -> Vec<String> {
    let Some(common) = locate(directory) else {
        return Vec::new();
    };
    let mut names = BTreeSet::new();
    for namespace in ["refs/heads", "refs/remotes", "refs/tags"] {
        collect_loose(&common.join(namespace), &mut names, MAX_REFERENCES);
    }
    if let Some(packed) = read_head(&common.join("packed-refs"), MAX_PACKED_BYTES) {
        for line in text_lines(&packed) {
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
