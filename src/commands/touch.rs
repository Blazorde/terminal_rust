use std::fs::OpenOptions;
use std::time::SystemTime;

use crate::errors::touch::TouchError;
use crate::utils::path::required_path;

pub fn touch(target: Option<&str>) -> Result<(), TouchError> {
    let path = required_path(target, TouchError::MissingFileName)?;

    if path.is_dir() {
        return Err(TouchError::TargetIsDirectory(path));
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| TouchError::CreateFailed {
            path: path.clone(),
            source,
        })?;
    // Opening an existing file does not change its date: set it, as touch does.
    file.set_modified(SystemTime::now())
        .map_err(|source| TouchError::CreateFailed { path, source })?;

    Ok(())
}

/// Touch every file of `targets`, even after a failure; return the failures.
pub fn touch_all(targets: &[String]) -> Vec<TouchError> {
    if targets.is_empty() {
        return vec![TouchError::MissingFileName];
    }
    targets
        .iter()
        .filter_map(|target| touch(Some(target)).err())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;
    use std::fs;

    #[test]
    fn touch_creates_file() {
        let path = temp_path("touch", "create");

        touch(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("touch should create file");

        assert!(path.is_file());

        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn touch_keeps_existing_file_content() {
        let path = temp_path("touch", "existing");
        fs::write(&path, "hello").expect("test file should be written");

        touch(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("touch should accept existing file");

        assert_eq!(
            fs::read_to_string(&path).expect("file should be readable"),
            "hello"
        );

        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn touch_updates_the_modification_time_of_an_existing_file() {
        let path = temp_path("touch", "mtime");
        fs::write(&path, "hello").expect("test file should be written");
        let old = SystemTime::now() - std::time::Duration::from_secs(3600);
        fs::File::options()
            .append(true)
            .open(&path)
            .and_then(|file| file.set_modified(old))
            .expect("the old date should be set");

        touch(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("touch should accept existing file");

        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .expect("the date should be readable");
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default();
        assert!(age < std::time::Duration::from_secs(60), "age {age:?}");
        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn touch_rejects_directory() {
        let path = temp_path("touch", "directory");
        fs::create_dir(&path).expect("test directory should be created");

        let error = touch(Some(path.to_str().expect("test path should be UTF-8")))
            .expect_err("touch should reject directories");

        assert!(matches!(error, TouchError::TargetIsDirectory(directory) if directory == path));

        fs::remove_dir(path).expect("test directory should be removed");
    }

    #[test]
    fn touch_requires_file_name() {
        let error = touch(None).expect_err("touch should require a file name");

        assert!(matches!(error, TouchError::MissingFileName));
    }
}
