use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::mkdir::MkdirError;

pub fn mkdir(target: Option<&str>) -> Result<(), MkdirError> {
    let path = target
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(PathBuf::from)
        .ok_or(MkdirError::MissingDirectoryName)?;

    create_directory(&path)
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
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after Unix epoch")
            .as_nanos();

        env::temp_dir().join(format!("terminal-rust-{name}-{unique}"))
    }

    #[test]
    fn mkdir_creates_directory() {
        let path = test_dir("create");

        mkdir(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("mkdir should create directory");

        assert!(path.is_dir());

        fs::remove_dir(path).expect("test directory should be removed");
    }

    #[test]
    fn mkdir_rejects_existing_directory() {
        let path = test_dir("exists");
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
