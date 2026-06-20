use std::fs;
use std::path::PathBuf;

use crate::errors::cat::CatError;
use crate::utils::path::{expand_home, trimmed_arg};

pub fn cat(target: Option<&str>) -> Result<String, CatError> {
    let path = resolve_target(target)?;

    if !path.exists() {
        return Err(CatError::NotFound(path));
    }

    if !path.is_file() {
        return Err(CatError::NotAFile(path));
    }

    fs::read_to_string(&path).map_err(|source| CatError::ReadFailed { path, source })
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
    fn cat_requires_file_name() {
        let error = cat(None).expect_err("cat should require a file name");

        assert!(matches!(error, CatError::MissingFileName));
    }
}
