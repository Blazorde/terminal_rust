use std::fs;
use std::path::PathBuf;

use crate::commands::system_fallback;
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

/// What `cat` prints for `targets` and the failures, in the order of the files.
/// With no target it copies the input of the pipeline, and `-` stands for that
/// input too. Every file is tried: a missing one does not hide the others.
pub fn cat_all(args: &[String]) -> (String, Vec<CatError>) {
    let targets: Vec<&String> = system_fallback::operands(args).collect();
    if targets.is_empty() {
        return match cat(None) {
            Ok(content) => (content, Vec::new()),
            Err(error) => (String::new(), vec![error]),
        };
    }
    let mut output = String::new();
    let mut errors = Vec::new();
    // The input of a pipeline can only be read once.
    let mut input = pipe_input::take();
    for target in targets {
        if target == "-" {
            if let Some(bytes) = input.take() {
                output.push_str(&String::from_utf8_lossy(&bytes));
            }
            continue;
        }
        match cat(Some(target)) {
            Ok(content) => output.push_str(&content),
            Err(error) => errors.push(error),
        }
    }
    (output, errors)
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
        // The input is read to its end: a second `cat` finds nothing more.
        assert_eq!(cat(None).as_deref().ok(), Some(""));
        // Without an input at all, `cat` still needs a file name.
        pipe_input::provide(None);
        assert!(matches!(cat(None), Err(CatError::MissingFileName)));
    }

    #[test]
    fn cat_with_an_empty_pipeline_input_prints_nothing() {
        pipe_input::provide(Some(Vec::new()));

        assert_eq!(cat(None).as_deref().ok(), Some(""));
    }

    #[test]
    fn cat_all_joins_several_files_and_reports_each_missing_one() {
        let first = temp_path("cat", "first");
        let second = temp_path("cat", "second");
        let missing = temp_path("cat", "missing");
        fs::write(&first, "one\n").expect("fixture");
        fs::write(&second, "two\n").expect("fixture");
        let names = |paths: &[&PathBuf]| -> Vec<String> {
            paths
                .iter()
                .map(|path| path.to_str().expect("UTF-8").to_string())
                .collect()
        };

        let (content, errors) = cat_all(&names(&[&first, &missing, &second]));

        assert_eq!(content, "one\ntwo\n");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(matches!(errors[0], CatError::NotFound(_)));
        fs::remove_file(first).expect("cleanup");
        fs::remove_file(second).expect("cleanup");
    }

    #[test]
    fn a_dash_stands_for_the_pipeline_input_between_files() {
        let file = temp_path("cat", "dash");
        fs::write(&file, "file\n").expect("fixture");
        pipe_input::provide(Some(b"piped\n".to_vec()));

        let (content, errors) = cat_all(&[
            "-".to_string(),
            file.to_str().expect("UTF-8").to_string(),
            "-".to_string(),
        ]);

        // The input is read once: the second dash has nothing left.
        assert_eq!(content, "piped\nfile\n");
        assert!(errors.is_empty());
        pipe_input::provide(None);
        fs::remove_file(file).expect("cleanup");
    }

    #[test]
    fn cat_requires_file_name() {
        pipe_input::provide(None);
        let error = cat(None).expect_err("cat should require a file name");

        assert!(matches!(error, CatError::MissingFileName));
    }
}
