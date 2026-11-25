use std::fs;
use std::path::Path;

use crate::commands::system_fallback;
use crate::errors::mkdir::MkdirError;
use crate::utils::path::required_path;

pub fn mkdir(target: Option<&str>) -> Result<(), MkdirError> {
    let path = required_path(target, MkdirError::MissingDirectoryName)?;

    create_directory(&path)
}

/// Create every directory of `targets`. With `-p` or `--parents`, before `--`, the
/// missing parents are created too and a directory that already exists is not an
/// error. The others are tried even if one fails; the failures are returned.
pub fn mkdir_all(args: &[String]) -> Vec<MkdirError> {
    // `-p` is the only option that reaches the built-in.
    let parents = system_fallback::options(args).next().is_some();
    let directories: Vec<&String> = system_fallback::operands(args).collect();
    if directories.is_empty() {
        return vec![MkdirError::MissingDirectoryName];
    }
    directories
        .into_iter()
        .filter_map(|target| {
            let result = if parents {
                create_with_parents(target)
            } else {
                mkdir(Some(target))
            };
            result.err()
        })
        .collect()
}

fn create_with_parents(target: &str) -> Result<(), MkdirError> {
    let path = required_path(Some(target), MkdirError::MissingDirectoryName)?;
    if path.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(&path).map_err(|source| MkdirError::CreateFailed { path, source })
}

fn create_directory(path: &Path) -> Result<(), MkdirError> {
    if path.exists() {
        return Err(MkdirError::AlreadyExists(path.to_path_buf()));
    }

    fs::create_dir(path).map_err(|source| MkdirError::CreateFailed {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn mkdir_all_creates_several_directories_and_keeps_going_after_a_failure() {
        let root = temp_path("mkdir", "several");
        fs::create_dir(&root).expect("root");
        let (first, existing, last) = (root.join("a"), root.join("b"), root.join("c"));
        fs::create_dir(&existing).expect("existing");

        let failures = mkdir_all(&strings(&[
            first.to_str().expect("UTF-8"),
            existing.to_str().expect("UTF-8"),
            last.to_str().expect("UTF-8"),
        ]));

        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(matches!(failures[0], MkdirError::AlreadyExists(_)));
        assert!(first.is_dir() && last.is_dir());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn mkdir_p_creates_parents_and_accepts_an_existing_directory() {
        let root = temp_path("mkdir", "parents");
        let deep = root.join("x").join("y").join("z");
        let path = deep.to_str().expect("UTF-8").to_string();

        assert!(mkdir_all(&["-p".to_string(), path.clone()]).is_empty());
        assert!(deep.is_dir());
        assert!(mkdir_all(&["-p".to_string(), path.clone()]).is_empty());
        // Without -p the missing parents are an error.
        let other = root.join("p").join("q");
        assert_eq!(
            mkdir_all(&strings(&[other.to_str().expect("UTF-8")])).len(),
            1
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn mkdir_all_without_a_name_is_a_usage_error() {
        assert!(matches!(
            mkdir_all(&[]).as_slice(),
            [MkdirError::MissingDirectoryName]
        ));
        assert!(matches!(
            mkdir_all(&strings(&["-p"])).as_slice(),
            [MkdirError::MissingDirectoryName]
        ));
    }

    #[test]
    fn mkdir_creates_directory() {
        let path = temp_path("mkdir", "create");

        mkdir(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("mkdir should create directory");

        assert!(path.is_dir());

        fs::remove_dir(path).expect("test directory should be removed");
    }

    #[test]
    fn mkdir_rejects_existing_directory() {
        let path = temp_path("mkdir", "exists");
        fs::create_dir(&path).expect("test directory should be created");

        let error = mkdir(Some(path.to_str().expect("test path should be UTF-8")))
            .expect_err("mkdir should reject existing directory");

        assert!(matches!(error, MkdirError::AlreadyExists(existing) if existing == path));

        fs::remove_dir(path).expect("test directory should be removed");
    }

    #[test]
    fn mkdir_requires_directory_name() {
        let error = mkdir(None).expect_err("mkdir should require a directory name");

        assert!(matches!(error, MkdirError::MissingDirectoryName));
    }
}
