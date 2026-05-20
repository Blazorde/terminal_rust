use std::fs::{self, Metadata};
use std::path::Path;

use crate::commands::ls::item::LsItem;
use crate::commands::ls::options::LsOptions;
use crate::errors::ls::LsError;

pub fn read_directory_items(path: &Path, options: &LsOptions) -> Result<Vec<LsItem>, LsError> {
    let mut items = Vec::new();

    let entries = fs::read_dir(path).map_err(|source| LsError::ReadDirectory {
        path: path.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| LsError::DirectoryEntry {
            path: path.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();

        if should_skip_entry(&name, options) {
            continue;
        }

        let path = entry.path();
        items.push(LsItem::from_directory_entry(
            path.clone(),
            metadata(&path)?,
            name,
        ));
    }

    Ok(items)
}

pub fn metadata(path: &Path) -> Result<Metadata, LsError> {
    fs::symlink_metadata(path).map_err(|source| LsError::Metadata {
        path: path.to_path_buf(),
        source,
    })
}

fn should_skip_entry(name: &str, options: &LsOptions) -> bool {
    !options.all && !options.almost_all && name.starts_with('.')
}
