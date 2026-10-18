use std::env;
use std::path::{Path, PathBuf};

use crate::errors::cd::CdError;
use crate::utils::path::{expand_home, trimmed_arg};

pub fn cd(target: Option<&str>, previous_dir: Option<&Path>) -> Result<PathBuf, CdError> {
    let current_dir = env::current_dir().map_err(CdError::CurrentDirectoryUnavailable)?;
    let path = resolve_target(target, previous_dir)?;

    if path.exists() && !path.is_dir() {
        return Err(CdError::NotADirectory(path));
    }

    env::set_current_dir(&path)
        .map_err(|source| CdError::ChangeDirectoryFailed { path, source })?;

    Ok(current_dir)
}

fn resolve_target(target: Option<&str>, previous_dir: Option<&Path>) -> Result<PathBuf, CdError> {
    match trimmed_arg(target) {
        Some("-") => previous_dir
            .map(Path::to_path_buf)
            .ok_or(CdError::PreviousDirectoryNotFound),
        Some(path) => expand_home(path).ok_or(CdError::HomeDirectoryNotFound),
        None => expand_home("~").ok_or(CdError::HomeDirectoryNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::cd::CdError;
    use crate::test_support::cwd_lock;
    use crate::utils::path::temp_path;
    use std::fs::{self, File};

    fn same_directory(left: &Path, right: &Path) -> bool {
        fs::canonicalize(left).ok() == fs::canonicalize(right).ok()
    }

    #[test]
    fn cd_changes_current_directory() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let target_dir = temp_path("cd", "directory");
        fs::create_dir(&target_dir).expect("target directory should be created");

        let previous_dir = cd(
            Some(target_dir.to_str().expect("target path should be UTF-8")),
            None,
        )
        .expect("cd should change the current directory");

        let current_dir = env::current_dir().expect("current dir should be readable");
        assert!(same_directory(&current_dir, &target_dir));
        assert_eq!(previous_dir, original_dir);

        env::set_current_dir(original_dir).expect("test should restore current directory");
        fs::remove_dir(target_dir).expect("target directory should be removed");
    }

    #[test]
    fn cd_rejects_files() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let file_path = temp_path("cd", "file.txt");

        File::create(&file_path).expect("test file should be created");
        let error = cd(
            Some(file_path.to_str().expect("file path should be UTF-8")),
            None,
        )
        .expect_err("cd should reject a file path");

        assert!(matches!(error, CdError::NotADirectory(path) if path == file_path));

        fs::remove_file(file_path).expect("test file should be removed");
        env::set_current_dir(original_dir).expect("test should restore current directory");
    }

    #[test]
    fn cd_dash_uses_previous_directory() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let target_dir = temp_path("cd", "previous-directory");
        fs::create_dir(&target_dir).expect("target directory should be created");

        env::set_current_dir(&target_dir).expect("test should enter target directory");
        cd(Some("-"), Some(&original_dir)).expect("cd - should use previous directory");

        let current_dir = env::current_dir().expect("current dir should be readable");
        assert!(same_directory(&current_dir, &original_dir));

        env::set_current_dir(original_dir).expect("test should restore current directory");
        fs::remove_dir(target_dir).expect("target directory should be removed");
    }
}
