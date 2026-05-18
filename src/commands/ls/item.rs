use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct LsItem {
    pub name: String,
    pub path: PathBuf,
    pub metadata: Metadata,
}

impl LsItem {
    pub fn from_path(path: &Path, metadata: Metadata) -> Self {
        Self {
            name: file_name(path),
            path: path.to_path_buf(),
            metadata,
        }
    }

    pub fn from_directory_entry(path: PathBuf, metadata: Metadata, name: String) -> Self {
        Self {
            name,
            path,
            metadata,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
