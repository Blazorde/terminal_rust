mod display;
mod fs;
mod item;
mod options;
mod sort;

use std::io;
use std::path::Path;

use crate::errors::ls::LsError;
use crate::render;

use self::display::print_item;
use self::fs::{metadata, read_directory_items};
use self::item::LsItem;
use self::options::LsOptions;
use self::sort::sort_items;

pub fn ls(args: &[String]) -> Result<(), LsError> {
    let (options, targets) = options::parse_args(args)?;
    let mut writer = Vec::new();

    if targets.is_empty() {
        list_path(Path::new("."), &options, &mut writer)?;
    } else {
        for (index, target) in targets.iter().enumerate() {
            display::print_target_header(target, index, targets.len(), &mut writer)?;
            list_path(target, &options, &mut writer)?;
        }
    }

    let output = String::from_utf8(writer)
        .map_err(|error| LsError::Output(io::Error::new(io::ErrorKind::InvalidData, error)))?;
    render::stdout(output).map_err(LsError::Output)
}

fn list_path(path: &Path, options: &LsOptions, writer: &mut impl io::Write) -> Result<(), LsError> {
    let metadata = metadata(path)?;

    if options.directory_itself || !metadata.is_dir() {
        let item = LsItem::from_path(path, metadata);
        print_item(&item, options, writer)?;
        return Ok(());
    }

    list_directory(path, options, writer)
}

fn list_directory(
    path: &Path,
    options: &LsOptions,
    writer: &mut impl io::Write,
) -> Result<(), LsError> {
    let mut items = read_directory_items(path, options)?;
    sort_items(&mut items, options);

    for item in &items {
        print_item(item, options, writer)?;
    }

    if options.recursive {
        for item in items {
            if item.metadata.is_dir() {
                display::print_recursive_header(&item.path, writer)?;
                list_directory(&item.path, options, writer)?;
            }
        }
    }

    Ok(())
}
