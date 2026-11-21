use std::env;
use std::path::{Path, PathBuf};

use crate::errors::cd::CdError;
use crate::utils::path::{expand_home, trimmed_arg};
use crate::utils::shell_env;

/// Change the working directory and return the one that was left, when it is
/// known. The current directory can have been deleted from elsewhere: that must
/// not stop the user from leaving it.
pub fn cd(target: Option<&str>, previous_dir: Option<&Path>) -> Result<Option<PathBuf>, CdError> {
    let current_dir = env::current_dir().ok();
    let path = resolve_target(target, previous_dir)?;

    if path.exists() && !path.is_dir() {
        return Err(CdError::NotADirectory(path));
    }

    env::set_current_dir(&path)
        .map_err(|source| CdError::ChangeDirectoryFailed { path, source })?;

    Ok(current_dir)
}

/// Tell the programs the shell starts where it is: `PWD` is the directory just
/// entered and `OLDPWD` the one that was left, when it is known.
pub fn record_change(left: Option<&Path>) {
    if let Some(left) = left {
        shell_env::export("OLDPWD", &left.to_string_lossy());
    }
    if let Ok(now) = env::current_dir() {
        shell_env::export("PWD", &now.to_string_lossy());
    }
}

/// What `PWD` should become when the shell starts in `current`, or `None` to keep
/// the inherited value. An inherited `PWD` that names the same directory by
/// another path (through a symbolic link) is kept, as other shells do; a stale
/// one, left by a parent that changed directory, is replaced.
fn initial_pwd(inherited: Option<PathBuf>, current: &Path) -> Option<PathBuf> {
    let same = |left: &Path, right: &Path| {
        matches!(
            (std::fs::canonicalize(left), std::fs::canonicalize(right)),
            (Ok(left), Ok(right)) if left == right
        )
    };
    match inherited {
        Some(path) if path.is_absolute() && same(&path, current) => None,
        _ => Some(current.to_path_buf()),
    }
}

/// Make `PWD` true when the shell starts, and forget the `OLDPWD` of the parent:
/// there is no previous directory yet.
pub fn start_in_current_directory() {
    if let Ok(current) = env::current_dir()
        && let Some(pwd) = initial_pwd(env::var_os("PWD").map(PathBuf::from), &current)
    {
        shell_env::export("PWD", &pwd.to_string_lossy());
    }
    shell_env::remove("OLDPWD");
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
    fn a_stale_pwd_is_replaced_and_a_link_to_the_directory_is_kept() {
        let directory = temp_path("cd", "initial-pwd");
        fs::create_dir(&directory).expect("fixture");
        let other = temp_path("cd", "initial-pwd-other");
        fs::create_dir(&other).expect("fixture");

        // Nothing inherited, or an inherited path that is another directory, or
        // one that does not exist, or a relative one: replaced by the real one.
        assert_eq!(initial_pwd(None, &directory), Some(directory.clone()));
        assert_eq!(
            initial_pwd(Some(other.clone()), &directory),
            Some(directory.clone())
        );
        assert_eq!(
            initial_pwd(
                Some(PathBuf::from("/terminal-rust/no/such/dir")),
                &directory
            ),
            Some(directory.clone())
        );
        assert_eq!(
            initial_pwd(Some(PathBuf::from("relative")), &directory),
            Some(directory.clone())
        );
        // The same directory: kept as inherited.
        assert_eq!(initial_pwd(Some(directory.clone()), &directory), None);
        #[cfg(unix)]
        {
            let link = temp_path("cd", "initial-pwd-link");
            std::os::unix::fs::symlink(&directory, &link).expect("symlink");
            assert_eq!(initial_pwd(Some(link.clone()), &directory), None);
            fs::remove_file(link).expect("cleanup");
        }

        fs::remove_dir(directory).expect("cleanup");
        fs::remove_dir(other).expect("cleanup");
    }

    #[test]
    fn a_directory_change_is_recorded_for_the_programs() {
        let guard = cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original_dir = env::current_dir().expect("current dir should be readable");
        let target_dir = temp_path("cd", "recorded");
        fs::create_dir(&target_dir).expect("fixture");
        let before = (shell_env::value("PWD"), shell_env::value("OLDPWD"));

        let left = cd(Some(target_dir.to_str().expect("UTF-8")), None).expect("cd");
        record_change(left.as_deref());
        let pwd = shell_env::value("PWD");
        let oldpwd = shell_env::value("OLDPWD");

        // Put everything back before asserting.
        env::set_current_dir(&original_dir).expect("restore cwd");
        for (name, value) in [("PWD", before.0), ("OLDPWD", before.1)] {
            match value {
                Some(value) => shell_env::export(name, &value),
                None => shell_env::remove(name),
            }
        }
        fs::remove_dir(&target_dir).expect("cleanup");
        drop(guard);

        assert!(same_directory(
            Path::new(&pwd.expect("PWD is set")),
            &target_dir
        ));
        assert_eq!(oldpwd, Some(original_dir.to_string_lossy().into_owned()));
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
        assert_eq!(previous_dir, Some(original_dir.clone()));

        env::set_current_dir(original_dir).expect("test should restore current directory");
        fs::remove_dir(target_dir).expect("target directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn cd_can_leave_a_directory_that_was_deleted() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current dir should be readable");
        let doomed = temp_path("cd", "deleted");
        fs::create_dir(&doomed).expect("directory should be created");
        env::set_current_dir(&doomed).expect("test should enter the directory");
        fs::remove_dir(&doomed).expect("directory should be removed");
        assert!(env::current_dir().is_err(), "the directory should be gone");

        let result = cd(
            Some(original_dir.to_str().expect("path should be UTF-8")),
            None,
        );

        let now = env::current_dir();
        env::set_current_dir(&original_dir).expect("test should restore current directory");
        assert!(matches!(result, Ok(None)), "{result:?}");
        assert!(now.is_ok_and(|dir| same_directory(&dir, &original_dir)));
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
