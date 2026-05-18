use std::env;
use std::path::PathBuf;

use crate::commands::cd_error::CdError;

pub fn cd(target: Option<&str>) -> Result<(), CdError> {
    let path = match target {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => home_directory().ok_or(CdError::HomeDirectoryNotFound)?,
    };

    if path.exists() && !path.is_dir() {
        return Err(CdError::NotADirectory(path));
    }

    env::set_current_dir(&path).map_err(|source| CdError::ChangeDirectoryFailed { path, source })
}

fn home_directory() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::cd_error::CdError;
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

        cd(Some(
            target_dir.to_str().expect("target path should be UTF-8"),
        ))
        .expect("cd should change the current directory");

        let current_dir = env::current_dir().expect("current dir should be readable");
        assert_eq!(current_dir, target_dir);

        env::set_current_dir(original_dir).expect("test should restore current directory");
    }

    #[test]
    fn cd_rejects_files() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let file_path = original_dir.join("target").join("cd-test-file.txt");

        File::create(&file_path).expect("test file should be created");
        let error = cd(Some(file_path.to_str().expect("file path should be UTF-8")))
            .expect_err("cd should reject a file path");

        assert!(matches!(error, CdError::NotADirectory(path) if path == file_path));

        fs::remove_file(file_path).expect("test file should be removed");
        env::set_current_dir(original_dir).expect("test should restore current directory");
    }
}
