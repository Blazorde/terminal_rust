//! The programs of `PATH`, offered where a command goes.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use super::matching::Case;
use crate::utils::shell_env;

/// How long the list of programs found in `PATH` is reused. Reading every
/// directory of `PATH` takes a moment, and `Tab` is pressed many times in a row.
const PROGRAM_CACHE_TTL: Duration = Duration::from_secs(10);

pub(super) struct ProgramCache {
    pub(super) path: OsString,
    pub(super) loaded: Instant,
    pub(super) names: Vec<String>,
}

/// The list of programs, read when first asked for.
#[derive(Default)]
pub(super) struct Programs {
    pub(super) cache: RefCell<Option<ProgramCache>>,
}

impl Programs {
    /// The programs of `PATH` that start with `prefix`, from a cache that is
    /// rebuilt when `PATH` changes or after a few seconds.
    pub(super) fn starting_with(&self, prefix: &str, case: Case) -> Vec<String> {
        let path = shell_env::value("PATH")
            .map(OsString::from)
            .unwrap_or_default();
        let mut cache = self.cache.borrow_mut();
        let stale = cache.as_ref().is_none_or(|cached| {
            cached.path != path || cached.loaded.elapsed() > PROGRAM_CACHE_TTL
        });
        if stale {
            let names = scan_path(&path);
            *cache = Some(ProgramCache {
                path,
                loaded: Instant::now(),
                names,
            });
        }
        cache
            .as_ref()
            .map(|cached| {
                cached
                    .names
                    .iter()
                    .filter(|name| case.starts_with(name, prefix))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The names of the executable files in the directories of `path`, sorted. On
/// Windows the extension that makes a file runnable (`.exe`) is left out, as the
/// shell finds the program without it.
pub(super) fn scan_path(path: &OsStr) -> Vec<String> {
    let mut names = BTreeSet::new();
    for directory in std::env::split_paths(path) {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if let Some(program) = program_name(&entry.path(), &name) {
                names.insert(program);
            }
        }
    }
    names.into_iter().collect()
}

#[cfg(unix)]
fn program_name(path: &Path, name: &str) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    // `metadata` follows symbolic links, and many programs are links.
    let metadata = fs::metadata(path).ok()?;
    (metadata.is_file() && metadata.permissions().mode() & 0o111 != 0).then(|| name.to_string())
}

#[cfg(windows)]
fn program_name(path: &Path, name: &str) -> Option<String> {
    const EXTENSIONS: [&str; 4] = [".exe", ".cmd", ".bat", ".com"];
    let lower = name.to_ascii_lowercase();
    let extension = EXTENSIONS
        .iter()
        .find(|extension| lower.ends_with(**extension))?;
    fs::metadata(path)
        .ok()?
        .is_file()
        .then(|| name[..name.len() - extension.len()].to_string())
}
