use std::fs::OpenOptions;

use crate::errors::touch::TouchError;
use crate::utils::path::required_path;

pub fn touch(target: Option<&str>) -> Result<(), TouchError> {
    let path = required_path(target, TouchError::MissingFileName)?;

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
