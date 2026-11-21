use std::fs;
use std::path::PathBuf;

use crate::errors::cat::CatError;
use crate::utils::path::{expand_home, trimmed_arg};
use crate::utils::pipe_input;

pub fn cat(target: Option<&str>) -> Result<String, CatError> {
    // Without a file name, `cat` copies the input of the pipeline it is in.
    if trimmed_arg(target).is_none()
        && let Some(bytes) = pipe_input::take()
    {
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }
    let path = resolve_target(target)?;

    if !path.exists() {
        return Err(CatError::NotFound(path));
    }

    if !path.is_file() {
        return Err(CatError::NotAFile(path));
    }

    // Bytes that are not valid UTF-8 (a binary file, a Latin-1 text) are shown as
    // U+FFFD instead of making the whole command fail.
    fs::read(&path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|source| CatError::ReadFailed { path, source })
}

fn resolve_target(target: Option<&str>) -> Result<PathBuf, CatError> {
    let target = trimmed_arg(target).ok_or(CatError::MissingFileName)?;

    expand_home(target).ok_or(CatError::HomeDirectoryNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;

    #[test]
    fn cat_reads_file_content() {
        let path = temp_path("cat", "file");
        fs::write(&path, "hello").expect("test file should be written");

        let content = cat(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("cat should read file content");

        assert_eq!(content, "hello");
        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn cat_shows_a_file_that_is_not_valid_utf8() {
        let path = temp_path("cat", "binary");
        fs::write(&path, b"caf\xe9 ok\xff\n").expect("test file should be written");

        let content = cat(Some(path.to_str().expect("test path should be UTF-8")))
            .expect("cat should not fail on invalid UTF-8");

        assert_eq!(content, "caf\u{fffd} ok\u{fffd}\n");
        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn cat_without_a_file_copies_the_pipeline_input() {
        pipe_input::provide(Some(b"from the pipe\n".to_vec()));

        assert_eq!(cat(None).as_deref().ok(), Some("from the pipe\n"));
        // The input is consumed: a second `cat` is a usage error again.
        assert!(matches!(cat(None), Err(CatError::MissingFileName)));
    }

    #[test]
    fn cat_with_an_empty_pipeline_input_prints_nothing() {
        pipe_input::provide(Some(Vec::new()));

        assert_eq!(cat(None).as_deref().ok(), Some(""));
    }

    #[test]
    fn cat_requires_file_name() {
        pipe_input::provide(None);
        let error = cat(None).expect_err("cat should require a file name");

        assert!(matches!(error, CatError::MissingFileName));
    }
}
