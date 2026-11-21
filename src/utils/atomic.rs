//! Replace a file so that a reader, or a crash, never sees half of it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Write `contents` to `path`, creating the parent directory when needed.
///
/// The data goes to a temporary file in the same directory, is flushed to disk,
/// then renamed over `path`. `rename` replaces the destination in one step on
/// Unix and on Windows, so the file is always either the old or the new version,
/// never missing and never partial. The temporary name is unique to the process
/// and the call, so two shells saving at once cannot corrupt each other's data.
/// An existing file keeps its permissions.
pub fn write_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?;
    fs::create_dir_all(parent)?;

    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));

    let result = write_and_replace(&temporary, path, contents);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    } else {
        sync_directory(parent);
    }
    result
}

fn write_and_replace(temporary: &Path, path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary)?;
    if let Ok(metadata) = fs::metadata(path) {
        // Best effort: keep a mode the user chose, such as 0600.
        let _ = file.set_permissions(metadata.permissions());
    }
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)
}

/// Make the rename itself durable. Not possible on every platform, and a failure
/// here does not undo a save that already succeeded.
fn sync_directory(directory: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(directory) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = directory;
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn directory(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "terminal-rust-atomic-{name}-{}-{}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should follow the Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("test directory should be created");
        path
    }

    fn entries(directory: &Path) -> Vec<String> {
        let mut names = fs::read_dir(directory)
            .expect("directory should be readable")
            .map(|entry| entry.expect("entry should be readable").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn creates_the_file_and_its_missing_parent_directories() {
        let root = directory("create");
        let path = root.join("a").join("b").join("state.toml");

        write_atomically(&path, b"one").expect("write should succeed");

        assert_eq!(fs::read(&path).expect("file should exist"), b"one");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn replaces_an_existing_file_and_leaves_nothing_behind() {
        let root = directory("replace");
        let path = root.join("state.toml");
        fs::write(&path, "old content that is longer").expect("fixture");

        write_atomically(&path, b"new").expect("write should succeed");

        assert_eq!(fs::read(&path).expect("file should exist"), b"new");
        assert_eq!(entries(&root), ["state.toml"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_failure_keeps_the_previous_file_and_removes_the_temporary_one() {
        let root = directory("failure");
        let path = root.join("state.toml");
        fs::write(&path, "kept").expect("fixture");
        // A directory cannot be replaced by a file: the rename fails.
        let blocked = root.join("blocked");
        fs::create_dir(&blocked).expect("fixture directory");
        fs::write(blocked.join("inside"), "x").expect("fixture file");

        assert!(write_atomically(&blocked, b"data").is_err());

        assert_eq!(fs::read(&path).expect("file should exist"), b"kept");
        assert_eq!(entries(&root), ["blocked", "state.toml"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn concurrent_writers_never_expose_a_partial_file() {
        let root = directory("concurrent");
        let path = root.join("state.toml");
        let contents = ["a".repeat(50_000), "b".repeat(70_000)];

        std::thread::scope(|scope| {
            for contents in &contents {
                let path = path.clone();
                scope.spawn(move || {
                    for _ in 0..40 {
                        write_atomically(&path, contents.as_bytes()).expect("write should succeed");
                    }
                });
            }
            let reader_path = path.clone();
            let contents = &contents;
            scope.spawn(move || {
                for _ in 0..400 {
                    if let Ok(read) = fs::read(&reader_path) {
                        assert!(
                            contents.iter().any(|expected| expected.as_bytes() == read),
                            "a partial file of {} bytes was visible",
                            read.len()
                        );
                    }
                }
            });
        });

        assert_eq!(entries(&root), ["state.toml"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn an_existing_file_keeps_its_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = directory("mode");
        let path = root.join("state.toml");
        fs::write(&path, "old").expect("fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");

        write_atomically(&path, b"new").expect("write should succeed");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_path_without_a_file_name_is_rejected() {
        assert!(write_atomically(Path::new("/"), b"x").is_err());
    }
}
