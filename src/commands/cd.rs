use std::env;
use std::path::{Path, PathBuf};

use crate::errors::cd::CdError;

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
    match target.map(str::trim).filter(|target| !target.is_empty()) {
        Some("-") => previous_dir
            .map(Path::to_path_buf)
            .ok_or(CdError::PreviousDirectoryNotFound),
        Some("~") => home_directory().ok_or(CdError::HomeDirectoryNotFound),
        Some(path) if path.starts_with("~/") || path.starts_with("~\\") => {
            let home = home_directory().ok_or(CdError::HomeDirectoryNotFound)?;
            Ok(home.join(&path[2..]))
        }
        Some(path) => Ok(PathBuf::from(path)),
        None => home_directory().ok_or(CdError::HomeDirectoryNotFound),
    }
}

fn home_directory() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::cd::CdError;
    use std::fs::{self, File};
    use std::sync::{Mutex, OnceLock};

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn cd_changes_current_directory() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let target_dir = original_dir.join("target");

        let previous_dir = cd(
            Some(target_dir.to_str().expect("target path should be UTF-8")),
            None,
        )
        .expect("cd should change the current directory");

        let current_dir = env::current_dir().expect("current dir should be readable");
        assert_eq!(current_dir, target_dir);
        assert_eq!(previous_dir, original_dir);

        env::set_current_dir(original_dir).expect("test should restore current directory");
    }

    #[test]
    fn cd_rejects_files() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let file_path = original_dir.join("target").join("cd-test-file.txt");

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
        let target_dir = original_dir.join("target");

        env::set_current_dir(&target_dir).expect("test should enter target directory");
        cd(Some("-"), Some(&original_dir)).expect("cd - should use previous directory");

        let current_dir = env::current_dir().expect("current dir should be readable");
        assert_eq!(current_dir, original_dir);

        env::set_current_dir(original_dir).expect("test should restore current directory");
    }
}
