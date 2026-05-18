use std::fs::OpenOptions;
use std::path::PathBuf;

use crate::errors::touch::TouchError;

pub fn touch(target: Option<&str>) -> Result<(), TouchError> {
    let path = target
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(PathBuf::from)
        .ok_or(TouchError::MissingFileName)?;

    if path.is_dir() {
        return Err(TouchError::TargetIsDirectory(path));
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| TouchError::CreateFailed { path, source })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after Unix epoch")
            .as_nanos();

        env::temp_dir().join(format!("terminal-rust-touch-{name}-{unique}"))
    }

    #[test]
    fn touch_creates_file() {
        let path = test_path("create");

        touch(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("touch should create file");

        assert!(path.is_file());

        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn touch_keeps_existing_file_content() {
        let path = test_path("existing");
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
    fn touch_rejects_directory() {
        let path = test_path("directory");
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
